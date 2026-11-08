package org.cuenora.android

import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test

class TransactionBrowserTest {
    private class Reader : HistoryReader {
        val queries = mutableListOf<TransactionQuery>()
        var result: RepositoryResult<Page<TransactionRecord>> = RepositoryResult.Success(Page(emptyList(), "next"))
        var accountPaging = false
        var pending: CompletableDeferred<RepositoryResult<Page<TransactionRecord>>>? = null
        override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
            queries += query
            return pending?.await() ?: result
        }
        override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page<AccountWithBalance>(emptyList(),
            if (accountPaging) { if (query.cursor == null) "accounts-next" else "accounts-last" } else null))
        override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
    }
    @Test fun appliesAndResetsServerFiltersWithoutReusingCursor() = runBlocking {
        val reader = Reader(); val browser = TransactionBrowser(reader)
        browser.loadIfNeeded(); browser.loadMore()
        browser.openFilters()
        browser.updateDraft(HistoryFilterDraft("2024-02-01", "2024-02-29", TransactionAccountFilter.Accountless))
        browser.applyFilters()
        assertEquals(BusinessDate(2024, 2, 1), reader.queries.last().dateFrom)
        assertEquals(BusinessDate(2024, 2, 29), reader.queries.last().dateTo)
        assertEquals(TransactionAccountFilter.Accountless, reader.queries.last().account)
        assertNull(reader.queries.last().cursor)
        browser.openFilters(); browser.updateDraft(HistoryFilterDraft("2024-02-30")); browser.applyFilters()
        assertNotNull(browser.state.value.filterError)
        assertEquals(3, reader.queries.size)
        browser.cancelFilters()
        assertEquals(TransactionAccountFilter.Accountless, browser.state.value.applied.account)
        browser.openFilters(); browser.resetDraft(); browser.applyFilters()
        assertNull(reader.queries.last().account); assertNull(reader.queries.last().dateFrom)
    }
    @Test fun paginationFailureRetriesSameCursorAndDenialClearsRows() = runBlocking {
        val reader = Reader(); val browser = TransactionBrowser(reader)
        browser.refresh()
        reader.result = RepositoryResult.Failure(ApiFailure.Network)
        browser.loadMore(); browser.loadMore()
        assertEquals(listOf(null, "next", "next"), reader.queries.map { it.cursor })
        assertTrue(browser.state.value.loaded)
        reader.result = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
        browser.loadMore()
        assertFalse(browser.state.value.loaded); assertTrue(browser.state.value.items.isEmpty())
    }
    @Test fun retiredBrowserIgnoresLateSuccessAndFailure() = runBlocking {
        for (failure in listOf(false, true)) {
            val reader = Reader(); reader.pending = CompletableDeferred()
            val browser = TransactionBrowser(reader)
            val job = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }
            browser.clear()
            reader.pending!!.complete(if (failure) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(Page(emptyList(), null)))
            job.join()
            assertEquals(TransactionBrowserState(), browser.state.value)
        }
    }
    @Test fun reopeningChoicesStartsANewCursorTraversal() = runBlocking {
        val reader = Reader(); reader.accountPaging = true
        val browser = TransactionBrowser(reader)
        browser.openFilters(); browser.loadChoices(); browser.loadChoices(more = true)
        assertFalse(browser.state.value.choicesFailure)
        browser.cancelFilters(); browser.openFilters(); browser.loadChoices()
        assertFalse(browser.state.value.choicesFailure)
        assertEquals("accounts-next", browser.state.value.accountCursor)
    }
    @Test fun cyclicCursorFailsWithoutLosingExistingPage() = runBlocking {
        val reader = Reader(); val browser = TransactionBrowser(reader)
        browser.refresh(); browser.loadMore()
        assertTrue(browser.state.value.failure is ApiFailure.InvalidResponse)
        assertEquals("next", browser.state.value.nextCursor)
    }
}
