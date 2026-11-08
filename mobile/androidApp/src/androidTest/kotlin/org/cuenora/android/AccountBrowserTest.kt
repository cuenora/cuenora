package org.cuenora.android

import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test

class AccountBrowserTest {
    private val owner = OwnerId(0, 7)
    private fun item(id: Long, amount: Long = 1234) = AccountWithBalance(Account(AccountId(0, id), owner,
        AccountName("Account $id"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE), Money(amount, Currency.SEK))
    private class Reader : AccountReader {
        val queries = mutableListOf<AccountListQuery>()
        var next: RepositoryResult<Page<AccountWithBalance>> = RepositoryResult.Success(Page(emptyList(), null))
        var gate: CompletableDeferred<Unit>? = null
        override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
            queries += query; gate?.await(); return next
        }
    }

    @Test fun paginationRetriesKeepCursorAndSelectionWithoutDuplicateRows() = runBlocking {
        val reader = Reader(); val browser = AccountBrowser(reader)
        reader.next = RepositoryResult.Success(Page(listOf(item(1)), "next")); browser.refresh()
        browser.select(AccountId(0, 1))
        reader.next = RepositoryResult.Failure(ApiFailure.Network); browser.loadMore()
        assertEquals(listOf(item(1)), browser.state.value.items)
        assertEquals(AccountId(0, 1), browser.state.value.selected?.account?.id)
        assertNotNull(browser.state.value.failure)
        reader.next = RepositoryResult.Success(Page(listOf(item(1), item(2, -1)), null)); browser.loadMore()
        assertEquals(listOf(1L, 2L), browser.state.value.items.map { it.account.id.lowBits })
        assertEquals(listOf(null, "next", "next"), reader.queries.map { it.cursor })
        browser.back(); assertNull(browser.state.value.selected)
        assertEquals(2, browser.state.value.items.size)
    }

    @Test fun failureIsNotEmptyRefreshDoesNotKeepStaleBalanceAndDeniedReadClearsData() = runBlocking {
        val reader = Reader(); val browser = AccountBrowser(reader)
        reader.next = RepositoryResult.Failure(ApiFailure.Timeout); browser.refresh()
        assertFalse(browser.state.value.loaded); assertNotNull(browser.state.value.failure)
        reader.next = RepositoryResult.Success(Page(emptyList(), null)); browser.refresh()
        assertTrue(browser.state.value.loaded); assertNull(browser.state.value.failure)
        reader.next = RepositoryResult.Success(Page(listOf(item(1)), null)); browser.refresh(); browser.select(AccountId(0, 1))
        reader.next = RepositoryResult.Failure(ApiFailure.Network); browser.refresh()
        assertTrue(browser.state.value.items.isEmpty()); assertNull(browser.state.value.selected)
        reader.next = RepositoryResult.Success(Page(listOf(item(1)), "next")); browser.refresh(); browser.select(AccountId(0, 1))
        reader.next = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null)); browser.loadMore()
        assertTrue(browser.state.value.items.isEmpty()); assertNull(browser.state.value.selected)
    }

    @Test fun duplicateLoadsCancellationAndDisposalCannotPublishStaleResults() = runBlocking {
        val reader = Reader(); val browser = AccountBrowser(reader)
        val gate = CompletableDeferred<Unit>(); reader.gate = gate
        reader.next = RepositoryResult.Success(Page(listOf(item(1)), null))
        val load = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }
        browser.refresh(); assertEquals(1, reader.queries.size)
        browser.clear(); gate.complete(Unit); load.join()
        assertTrue(browser.state.value.items.isEmpty()); assertFalse(browser.state.value.loaded)
        val newBrowser = AccountBrowser(reader)
        reader.gate = CompletableDeferred()
        val cancelled = launch(start = CoroutineStart.UNDISPATCHED) { newBrowser.refresh() }
        cancelled.cancelAndJoin(); assertFalse(newBrowser.state.value.busy)
        reader.gate = null; newBrowser.refresh(); assertTrue(newBrowser.state.value.loaded)
    }

    @Test fun exactAmountRenderingCoversSignsZeroAndSymmetricBounds() {
        assertEquals("0.00", accountAmount(Money(0, Currency.SEK)))
        assertEquals("-0.01", accountAmount(Money(-1, Currency.USD)))
        assertEquals("12.34", accountAmount(Money(1234, Currency.EUR)))
        assertEquals("92233720368547758.07", accountAmount(Money(Long.MAX_VALUE, Currency.SEK)))
        assertEquals("-92233720368547758.07", accountAmount(Money(-Long.MAX_VALUE, Currency.SEK)))
    }
}
