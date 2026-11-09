package org.cuenora.android
import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test

class HistoryBoundaryBrowserTest {
    private val row = TransactionRecord(Transaction(TransactionId(0,1), TransactionKind.EXPENSE, TransactionState.POSTED,
        Money(12345, Currency.SEK), BusinessDate(2024,2,29)))
    private class Reader : HistoryReader {
        val queries = mutableListOf<TransactionQuery>()
        var listing: suspend (TransactionQuery) -> RepositoryResult<Page<TransactionRecord>> = { RepositoryResult.Success(Page(emptyList(), null)) }
        var choices: suspend () -> RepositoryResult<Page<AccountWithBalance>> = { RepositoryResult.Success(Page(emptyList(), null)) }
        var catalog: suspend () -> RepositoryResult<List<CatalogCategory>> = { RepositoryResult.Success(emptyList()) }
        override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> { queries += query; return listing(query) }
        override suspend fun accounts(query: AccountListQuery) = choices()
        override suspend fun categories() = catalog()
    }
    @Test fun independentLateReadsCannotRepublishAfterClear() = runBlocking {
        for (target in listOf("history", "accounts", "categories")) for (failed in listOf(false,true)) {
            val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
            val reader = Reader()
            val failure = RepositoryResult.Failure(ApiFailure.Http(403,null,null,null))
            reader.listing = { entered.complete(Unit); release.await(); if(failed) failure else RepositoryResult.Success(Page(listOf(row), "next")) }
            if(target == "accounts") reader.choices = { entered.complete(Unit); release.await(); if(failed) failure else RepositoryResult.Success(Page(emptyList(), "next")) }
            if(target == "categories") reader.catalog = { entered.complete(Unit); release.await(); if(failed) failure else RepositoryResult.Success(listOf(CatalogCategory(CategoryId(0,1),"food",TransactionKind.EXPENSE,"Food"))) }
            val browser = TransactionBrowser(reader)
            val job = launch { if(target == "history") browser.refresh() else browser.loadChoices() }
            entered.await(); browser.clear(); release.complete(Unit); job.join()
            assertEquals("$target failure=$failed", TransactionBrowserState(), browser.state.value)
            browser.refresh(); browser.loadMore(); browser.openFilters(); browser.loadChoices()
            assertEquals(TransactionBrowserState(), browser.state.value)
        }
    }
    @Test fun independentFiltersAreServerQueriesNeverLocalPageFiltering() = runBlocking {
        val reader = Reader(); reader.listing = { q -> RepositoryResult.Success(Page(listOf(row), if(q.cursor == null) "more" else null)) }
        val browser = TransactionBrowser(reader); browser.refresh(); browser.loadMore()
        assertEquals(listOf(null,"more"),reader.queries.map { it.cursor })
        browser.openFilters(); browser.updateDraft(HistoryFilterDraft("2025-01-01",account=TransactionAccountFilter.Accountless,category=CategoryId(0,3)))
        browser.cancelFilters(); assertEquals(2,reader.queries.size); assertEquals(HistoryFilterDraft(),browser.state.value.applied)
        browser.openFilters(); browser.updateDraft(HistoryFilterDraft("2025-01-01",account=TransactionAccountFilter.Accountless,category=CategoryId(0,3))); browser.applyFilters()
        assertNull(reader.queries.last().cursor); assertEquals(BusinessDate(2025,1,1),reader.queries.last().dateFrom)
        assertEquals(CategoryId(0,3),reader.queries.last().categoryId); assertEquals(TransactionAccountFilter.Accountless, reader.queries.last().account)
        assertEquals(listOf(row),browser.state.value.items) // adapter must not locally trim a finite server page
        browser.loadMore(); assertEquals("more",reader.queries.last().cursor); assertEquals(1,browser.state.value.items.size)
        browser.openFilters(); browser.resetDraft(); browser.applyFilters()
        assertEquals(TransactionQuery(limit=20),reader.queries.last()); assertEquals(listOf(row),browser.state.value.items)
    }
    @Test fun independentDeniedChoicesEraseSensitiveRowsAndChoices() = runBlocking {
        val reader=Reader(); reader.listing={ RepositoryResult.Success(Page(listOf(row),null)) }
        val browser=TransactionBrowser(reader); browser.refresh()
        reader.catalog={ RepositoryResult.Failure(ApiFailure.Http(403,null,null,null)) }
        browser.openFilters(); browser.loadChoices()
        assertEquals(TransactionBrowserState(failure=ApiFailure.Http(403,null,null,null)),browser.state.value)
    }
}
