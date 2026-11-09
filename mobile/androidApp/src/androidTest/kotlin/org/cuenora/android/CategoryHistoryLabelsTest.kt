package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class CategoryHistoryLabelsTest {
    @get:Rule val compose = createComposeRule()
    private val categoryId = CategoryId(0, 9)
    private val catalog = listOf(CatalogCategory(categoryId, "groceries", TransactionKind.EXPENSE, "Groceries"))
    private fun event(category: CategoryId? = categoryId) = TransactionRecord(Transaction(
        TransactionId(0, 1), TransactionKind.EXPENSE, TransactionState.POSTED,
        Money(1000, Currency.SEK), BusinessDate(2024, 2, 29), categoryId = category))
    private fun reader(
        category: CategoryId? = categoryId,
        categories: suspend () -> RepositoryResult<List<CatalogCategory>> = { RepositoryResult.Success(catalog) },
    ) = object : HistoryReader {
        override suspend fun list(query: TransactionQuery) = RepositoryResult.Success(Page(listOf(event(category)), null))
        override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page(emptyList<AccountWithBalance>(), null))
        override suspend fun categories() = categories.invoke()
    }

    @Test fun rendersCategoryNameWithoutOpeningFilters() {
        val browser = TransactionBrowser(reader())
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { TransactionsScreen(browser, { action -> scope.launch { action() } }, {}) }
        }
        compose.onNodeWithText("Category: Groceries").performScrollTo().assertExists()
        compose.onNodeWithText("00000000-0000-0000-0000-000000000009", substring = true).assertDoesNotExist()
    }

    @Test fun failedOrMissingCategoryUsesSafeLabelNotUuid() {
        val browser = TransactionBrowser(reader(categories = { RepositoryResult.Failure(ApiFailure.Network) }))
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { TransactionsScreen(browser, { action -> scope.launch { action() } }, {}) }
        }
        compose.onNodeWithText("Category: Category unavailable").performScrollTo().assertExists()
        compose.onNodeWithText("00000000-0000-0000-0000-000000000009", substring = true).assertDoesNotExist()
    }

    @Test fun refreshRetriesFailedCatalogAndFiltersStillUseCategoryIdentity() = runBlocking {
        var reads = 0
        val browser = TransactionBrowser(reader(categories = {
            reads++
            if (reads == 1) RepositoryResult.Failure(ApiFailure.Network) else RepositoryResult.Success(catalog)
        }))
        browser.loadIfNeeded()
        assertTrue(browser.state.value.loaded)
        assertTrue(browser.state.value.categories.isEmpty())
        browser.refresh()
        assertEquals(catalog, browser.state.value.categories)
        assertEquals(2, reads)
        browser.openFilters(); browser.loadChoices()
        browser.updateDraft(HistoryFilterDraft(category = categoryId))
        browser.applyFilters()
        assertEquals(categoryId, browser.state.value.applied.category)
        assertEquals(catalog, browser.state.value.categories)
    }

    @Test fun categoryLookupDenialClearsRecordsAndRetirementIgnoresLateResults() = runBlocking {
        for (status in listOf(401, 403)) {
            val browser = TransactionBrowser(reader(categories = {
                RepositoryResult.Failure(ApiFailure.Http(status, null, null, null))
            }))
            browser.loadIfNeeded()
            assertTrue(browser.state.value.items.isEmpty())
            assertFalse(browser.state.value.loaded)
            assertEquals(status, (browser.state.value.failure as ApiFailure.Http).status)
        }
        for (failed in listOf(false, true)) {
            val pending = CompletableDeferred<RepositoryResult<List<CatalogCategory>>>()
            val browser = TransactionBrowser(reader(categories = { pending.await() }))
            val job = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }
            browser.clear()
            pending.complete(if (failed) RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                else RepositoryResult.Success(catalog))
            job.join()
            assertEquals(TransactionBrowserState(), browser.state.value)
        }
    }

    @Test fun categorylessRecordsDoNotRequireCatalog() = runBlocking {
        val browser = TransactionBrowser(reader(category = null, categories = { error("No category lookup needed") }))
        browser.loadIfNeeded()
        assertTrue(browser.state.value.loaded)
        assertEquals(1, browser.state.value.items.size)
        assertFalse(browser.state.value.busy)
    }
}
