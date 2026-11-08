package org.cuenora.android

import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test

/** Retained independent probes for signed-in browser retirement and pagination isolation. */
class IndependentBrowserProbeTest {
    private fun item(id: Long, owner: Long = 7) = AccountWithBalance(
        Account(AccountId(0, id), OwnerId(0, owner), AccountName("Account $id"),
            AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE), Money(-Long.MAX_VALUE, Currency.SEK),
    )

    @Test fun lateNonCooperativeResultCannotRestoreDisposedData() = runBlocking {
        val gate = CompletableDeferred<Unit>()
        var calls = 0
        val browser = AccountBrowser(object : AccountReader {
            override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                calls++
                try { withContext(NonCancellable) { gate.await() } }
                catch (_: CancellationException) {
                    // Deliberately non-cooperative dependency returns even after cancellation.
                }
                return RepositoryResult.Success(Page(listOf(item(1)), null))
            }
        })
        val pending = launch(start = CoroutineStart.UNDISPATCHED) { browser.refresh() }
        browser.refresh(); browser.loadMore()
        assertEquals(1, calls)
        browser.clear(); pending.cancel(); gate.complete(Unit); pending.join()
        browser.refresh(); browser.select(AccountId(0, 1))
        assertEquals(1, calls)
        assertEquals(AccountBrowserState(), browser.state.value)
    }

    @Test fun cursorCyclesMixedOwnersAndDeniedPagesFailWithoutSensitiveRows() = runBlocking {
        var next: RepositoryResult<Page<AccountWithBalance>> = RepositoryResult.Success(Page(listOf(item(1)), "a"))
        val browser = AccountBrowser(object : AccountReader {
            override suspend fun list(query: AccountListQuery) = next
        })
        browser.refresh(); browser.select(AccountId(0, 1))
        next = RepositoryResult.Success(Page(listOf(item(2)), "b")); browser.loadMore()
        next = RepositoryResult.Success(Page(listOf(item(3)), "a")); browser.loadMore()
        assertTrue(browser.state.value.failure is ApiFailure.InvalidResponse)
        assertEquals(2, browser.state.value.items.size)
        assertEquals("b", browser.state.value.nextCursor)
        next = RepositoryResult.Success(Page(listOf(item(4, 8)), null)); browser.loadMore()
        assertEquals(403, (browser.state.value.failure as ApiFailure.Http).status)
        assertTrue(browser.state.value.items.isEmpty()); assertNull(browser.state.value.selected)
        next = RepositoryResult.Success(Page(listOf(item(1)), "a")); browser.refresh()
        next = RepositoryResult.Failure(ApiFailure.Http(401, null, null, null)); browser.loadMore()
        assertTrue(browser.state.value.items.isEmpty()); assertNull(browser.state.value.nextCursor)
        assertEquals("-92233720368547758.07", accountAmount(item(1).balance))
    }
}
