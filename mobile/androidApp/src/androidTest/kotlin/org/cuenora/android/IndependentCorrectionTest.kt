package org.cuenora.android

import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test

class IndependentCorrectionTest {
    private val source = AccountId(0, 2)
    private val destination = AccountId(0, 3)
    private val transfer = TransactionRecord(Transaction(
        TransactionId(0, 1), TransactionKind.TRANSFER, TransactionState.POSTED,
        Money(100, Currency.SEK), BusinessDate(2024, 1, 1), accountId = source,
        destinationAccountId = destination, receivedAmount = Money(90, Currency.EUR),
    ))
    private fun account(id: AccountId, name: String) = AccountWithBalance(
        Account(id, OwnerId(0, 7), AccountName(name), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE),
        Money(0, Currency.SEK),
    )
    private class Reader : HistoryReader {
        var listing: suspend (TransactionQuery) -> RepositoryResult<Page<TransactionRecord>> = {
            RepositoryResult.Success(Page(emptyList(), null))
        }
        var metadata: suspend (AccountListQuery) -> RepositoryResult<Page<AccountWithBalance>> = {
            RepositoryResult.Success(Page(emptyList(), null))
        }
        override suspend fun list(query: TransactionQuery) = listing(query)
        override suspend fun accounts(query: AccountListQuery) = metadata(query)
        override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
    }

    @Test fun metadataDenialClearsPreviouslyResolvedLabelsAndRows() = runBlocking {
        for (status in listOf(401, 403)) {
            val reader = Reader()
            var listingCalls = 0
            var denied = false
            reader.listing = {
                listingCalls++
                val row = if (listingCalls == 1) TransactionRecord(Transaction(
                    TransactionId(0, 9), TransactionKind.EXPENSE, TransactionState.POSTED,
                    Money(100, Currency.SEK), BusinessDate(2024, 1, 1), accountId = source,
                )) else transfer
                RepositoryResult.Success(Page(listOf(row), if (listingCalls == 1) "next" else null))
            }
            reader.metadata = {
                if (denied) RepositoryResult.Failure(ApiFailure.Http(status, null, null, null))
                else RepositoryResult.Success(Page(listOf(account(source, "Source")), "remaining"))
            }
            val browser = TransactionBrowser(reader)
            browser.refresh()
            assertEquals("Source", browser.state.value.accountLabels[source])
            denied = true
            browser.loadMore()
            assertEquals(TransactionBrowserState(failure = ApiFailure.Http(status, null, null, null)), browser.state.value)
        }
    }

    @Test fun secondMetadataPageCannotPublishAfterClearSuccessOrFailure() = runBlocking {
        for (failed in listOf(false, true)) {
            val reader = Reader()
            val entered = CompletableDeferred<Unit>()
            val release = CompletableDeferred<Unit>()
            reader.listing = { RepositoryResult.Success(Page(listOf(transfer), null)) }
            reader.metadata = { query ->
                if (query.cursor == null) RepositoryResult.Success(Page(listOf(account(source, "Source")), "last"))
                else {
                    entered.complete(Unit)
                    release.await()
                    if (failed) RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                    else RepositoryResult.Success(Page(listOf(account(destination, "Destination")), null))
                }
            }
            val browser = TransactionBrowser(reader)
            val job = launch { browser.refresh() }
            entered.await()
            assertTrue(browser.state.value.busy)
            browser.openFilters()
            browser.updateDraft(HistoryFilterDraft(account = TransactionAccountFilter.Accountless))
            browser.applyFilters()
            assertEquals(HistoryFilterDraft(), browser.state.value.applied)
            assertFalse(browser.state.value.filtersOpen)
            browser.clear()
            release.complete(Unit)
            job.join()
            assertEquals(TransactionBrowserState(), browser.state.value)
        }
    }

    @Test fun paginationAndRefreshKeepServerRowsAndReloadBothEndpointNames() = runBlocking {
        val reader = Reader()
        var suffix = "Old"
        val queries = mutableListOf<TransactionQuery>()
        val accountQueries = mutableListOf<String?>()
        reader.listing = { query ->
            queries += query
            RepositoryResult.Success(Page(listOf(transfer), if (query.cursor == null) "next" else null))
        }
        reader.metadata = { query ->
            accountQueries += query.cursor
            if (query.cursor == null) RepositoryResult.Success(Page(listOf(account(source, "Source $suffix")), "last"))
            else RepositoryResult.Success(Page(listOf(account(destination, "Destination $suffix")), null))
        }
        val browser = TransactionBrowser(reader)
        browser.refresh()
        browser.loadMore()
        assertEquals(listOf(transfer), browser.state.value.items)
        assertEquals(listOf(null, "next"), queries.map { it.cursor })
        assertEquals("Destination Old", browser.state.value.accountLabels[destination])
        suffix = "New"
        browser.refresh()
        assertEquals("Source New", browser.state.value.accountLabels[source])
        assertEquals("Destination New", browser.state.value.accountLabels[destination])
        assertEquals(listOf(null, "last", null, "last"), accountQueries)
        browser.openFilters()
        browser.updateDraft(HistoryFilterDraft("2025-01-01", account = TransactionAccountFilter.Accountless))
        browser.applyFilters()
        assertEquals(listOf(transfer), browser.state.value.items)
        assertNull(queries.last().cursor)
        assertEquals(TransactionAccountFilter.Accountless, queries.last().account)
    }
}
