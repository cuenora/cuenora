package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollTo
import kotlinx.coroutines.launch
import kotlinx.coroutines.*
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.android.ui.*
import org.cuenora.android.ui.TransactionsScreen
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.repository.*
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.*

class AccountHistoryLabelsTest {
    @get:Rule val compose = createComposeRule()

    private val sourceId = AccountId(0, 2)
    private val destinationId = AccountId(0, 3)
    private fun account(id: AccountId, name: String, currency: Currency, status: AccountStatus = AccountStatus.ACTIVE) =
        AccountWithBalance(Account(id, OwnerId(0, 7), AccountName(name),
            if (currency == Currency.SEK) AccountType.NORMAL else AccountType.SAVINGS, currency, status), Money(0, currency))
    private fun transfer() = TransactionRecord(Transaction(TransactionId(0, 1), TransactionKind.TRANSFER,
        TransactionState.POSTED, Money(1000, Currency.SEK), BusinessDate(2024, 2, 29),
        accountId = sourceId, destinationAccountId = destinationId, receivedAmount = Money(900, Currency.EUR)))
    private fun reader(
        accountPage: suspend (AccountListQuery) -> RepositoryResult<Page<AccountWithBalance>> = { query ->
            if (query.cursor == null) RepositoryResult.Success(Page(listOf(account(sourceId, "Daily account", Currency.SEK)), "accounts-next"))
            else RepositoryResult.Success(Page(listOf(account(destinationId, "Archived savings", Currency.EUR, AccountStatus.ARCHIVED)), null))
        },
    ) = object : HistoryReader {
        override suspend fun list(query: TransactionQuery) = RepositoryResult.Success(Page(listOf(transfer()), null))
        override suspend fun accounts(query: AccountListQuery) = accountPage(query)
        override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
    }

    @Test fun resolvesVisibleAccountsAcrossPagesAndKeepsLabelsWhenChoicesReload() = runBlocking {
        val queries = mutableListOf<AccountListQuery>()
        val browser = TransactionBrowser(reader { query ->
            queries += query
            if (query.cursor == null) RepositoryResult.Success(Page(listOf(account(sourceId, "Daily account", Currency.SEK)), "accounts-next"))
            else RepositoryResult.Success(Page(listOf(account(destinationId, "Archived savings", Currency.EUR, AccountStatus.ARCHIVED)), null))
        })

        browser.loadIfNeeded()
        assertEquals("Daily account", browser.state.value.accountLabels[sourceId])
        assertEquals("Archived savings", browser.state.value.accountLabels[destinationId])
        assertEquals(listOf(null, "accounts-next"), queries.map { it.cursor })

        browser.openFilters(); browser.loadChoices(); browser.loadChoices(more = true)
        browser.cancelFilters(); browser.openFilters(); browser.loadChoices()
        assertEquals("Archived savings", browser.state.value.accountLabels[destinationId])
        assertEquals(listOf(null, "accounts-next", null, "accounts-next", null), queries.map { it.cursor })
    }

    @Test fun unavailableMetadataDoesNotExposeUuidAndRefreshRetriesLookup() = runBlocking {
        var fail = true
        var accountReads = 0
        val browser = TransactionBrowser(reader { query ->
            accountReads++
            if (fail) RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.Network)
            else if (query.cursor == null) RepositoryResult.Success(Page(listOf(account(sourceId, "Daily account", Currency.SEK)), null))
            else error("Unexpected account page")
        })
        browser.loadIfNeeded()
        assertTrue(browser.state.value.loaded)
        assertTrue(browser.state.value.accountLabels.isEmpty())
        assertEquals("Account unavailable", browser.state.value.accountLabels[sourceId] ?: "Account unavailable")
        assertEquals(1, accountReads)
        fail = false
        browser.refresh()
        assertEquals("Daily account", browser.state.value.accountLabels[sourceId])
        assertEquals(2, accountReads)
    }

    @Test fun clearRetiresSuccessfulAndFailedLateMetadataReads() = runBlocking {
        for (failure in listOf(false, true)) {
            val pending = CompletableDeferred<RepositoryResult<Page<AccountWithBalance>>>()
            val base = reader { pending.await() }
            val browser = TransactionBrowser(base)
            val job = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }
            browser.clear()
            pending.complete(if (failure) RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.Http(403, null, null, null))
                else RepositoryResult.Success(Page(listOf(account(sourceId, "Daily account", Currency.SEK)), null)))
            job.join()
            assertEquals(TransactionBrowserState(), browser.state.value)
        }
    }

    @Test fun keepsBrowserBusyUntilDeferredMetadataReadCompletes() = runBlocking {
        val pending = CompletableDeferred<RepositoryResult<Page<AccountWithBalance>>>()
        val browser = TransactionBrowser(reader { pending.await() })
        val job = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }

        browser.openFilters()
        browser.updateDraft(HistoryFilterDraft(account = TransactionAccountFilter.Accountless))
        browser.applyFilters()
        val duringMetadataRead = browser.state.value

        pending.complete(RepositoryResult.Success(Page(listOf(account(sourceId, "Daily account", Currency.SEK)), null)))
        job.join()
        assertEquals(HistoryFilterDraft(), duringMetadataRead.applied)
        assertTrue(duringMetadataRead.busy)
        assertFalse(duringMetadataRead.filtersOpen)
        assertTrue(duringMetadataRead.items.isNotEmpty())
        assertFalse(browser.state.value.busy)
        assertEquals(HistoryFilterDraft(), browser.state.value.applied)
    }

    @Test fun explicitRefreshReloadsAccountNamesAfterServerRename() = runBlocking {
        var accountReads = 0
        val event = TransactionRecord(Transaction(TransactionId(0, 1), TransactionKind.EXPENSE,
            TransactionState.POSTED, Money(1000, Currency.SEK), BusinessDate(2024, 2, 29), accountId = sourceId))
        val historyReader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery) = RepositoryResult.Success(Page(listOf(event), null))
            override suspend fun accounts(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                accountReads++
                val name = if (accountReads == 1) "Daily account" else "Renamed account"
                return RepositoryResult.Success(Page(listOf(account(sourceId, name, Currency.SEK)), null))
            }
            override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
        }
        val browser = TransactionBrowser(historyReader)

        browser.loadIfNeeded()
        assertEquals("Daily account", browser.state.value.accountLabels[sourceId])
        browser.refresh()

        assertEquals(2, accountReads)
        assertEquals("Renamed account", browser.state.value.accountLabels[sourceId])
    }

    @Test fun rendersAccountNamesBeforeFiltersAreOpenedIncludingTransferDestination() {
        val reader = reader()
        val browser = TransactionBrowser(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { TransactionsScreen(browser, { action -> scope.launch { action() } }, {}) }
        }

        compose.onNodeWithText("Account: Daily account").performScrollTo().assertExists()
        compose.onNodeWithText("Destination: Archived savings").performScrollTo().assertExists()
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.onNodeWithText("More account choices").performScrollTo().performClick()
        compose.onNodeWithText("Archived savings · EUR · Archived").performScrollTo().performClick()
        compose.onNodeWithText("Apply filters").performClick()
        compose.onNodeWithText("Account Archived savings", substring = true).performScrollTo().assertExists()
    }

    @Test fun unavailableNamesUseSafeFallbackInsteadOfAccountUuid() {
        val browser = TransactionBrowser(reader { RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.Network) })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { TransactionsScreen(browser, { action -> scope.launch { action() } }, {}) }
        }
        compose.onNodeWithText("Account: Account unavailable").performScrollTo().assertExists()
        compose.onNodeWithText("00000000-0000-0000-0000-000000000002").assertDoesNotExist()
    }
}
