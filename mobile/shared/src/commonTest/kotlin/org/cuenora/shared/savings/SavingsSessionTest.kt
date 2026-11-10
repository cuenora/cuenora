package org.cuenora.shared.savings

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import kotlin.test.*

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class SavingsSessionTest {
    private val owner = OwnerId(0, 1)
    private val account = Account(AccountId(0, 2), owner, AccountName("Savings"), AccountType.SAVINGS, Currency.SEK, AccountStatus.ACTIVE)
    private val receipt = SavingsAction.Receipt(account.id, "12.34", Currency.SEK, "2026-11-10", "Actual bank credit")
    private class Journal : SavingsJournal {
        var pending: PendingSavings? = null
        var gate: CompletableDeferred<Unit>? = null
        override suspend fun read(owner: OwnerId, origin: String) = pending
        override suspend fun save(owner: OwnerId, origin: String, pending: PendingSavings) { gate?.await(); this.pending = pending }
        override suspend fun clear(owner: OwnerId, origin: String) { pending = null }
    }
    private inner class Backend : SavingsBackend {
        var calls = 0
        var result: RepositoryResult<Unit> = RepositoryResult.Failure(ApiFailure.Network)
        var extraAccounts = emptyList<Account>()
        var identity = OnboardingProfile(owner, true, "Europe/Stockholm")
        override suspend fun profile() = RepositoryResult.Success(identity)
        override suspend fun accounts(owner: OwnerId) = RepositoryResult.Success(listOf(account) + extraAccounts)
        override suspend fun terms(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery) = RepositoryResult.Success(Page(emptyList<SavingsTermsVersion>(), null))
        override suspend fun rates(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery) = RepositoryResult.Success(Page(emptyList<SavingsRateVersion>(), null))
        override suspend fun forecast(owner: OwnerId, account: AccountId, start: BusinessDate, end: BusinessDate): RepositoryResult<SavingsEstimate> = RepositoryResult.Failure(ApiFailure.Network)
        override suspend fun write(pending: PendingSavings): RepositoryResult<Unit> { calls++; return result }
    }
    private fun flow(backend: Backend, journal: Journal) = SavingsSession(backend, journal, "https://example.invalid", { IdempotencyKey("original-key") }, { "2026-11-10" })

    @Test fun actualReceiptRequiresReviewAndRestoresOriginalWithoutReplay() = runTest {
        val backend = Backend(); val journal = Journal(); val flow = flow(backend, journal)
        flow.open(account); flow.confirm(); assertEquals(0, backend.calls)
        flow.review(receipt); assertEquals(SavingsStage.Review, flow.state.value.stage)
        flow.confirm(); assertEquals(SavingsStage.Uncertain, flow.state.value.stage)
        val original = journal.pending!!.encode()
        flow.close()
        val restored = flow(backend, journal); restored.open(account)
        assertEquals(1, backend.calls); assertEquals(original, journal.pending!!.encode())
        restored.review(receipt.copy(amount = "99.00")); assertEquals(receipt, restored.state.value.action)
        backend.result = RepositoryResult.Success(Unit); restored.confirm()
        assertEquals(2, backend.calls); assertNull(journal.pending)
        assertEquals(SavingsStage.Saved, restored.state.value.stage)
    }
    @Test fun retirementDuringStoragePreventsLateWriteButKeepsIntent() = runTest {
        val backend = Backend(); val journal = Journal(); val flow = flow(backend, journal)
        flow.open(account); flow.review(receipt)
        journal.gate = CompletableDeferred()
        val saving = launch { flow.confirm() }; runCurrent()
        flow.clear(); journal.gate!!.complete(Unit); saving.join()
        assertEquals(0, backend.calls); assertNotNull(journal.pending)
        assertEquals(SavingsStage.Closed, flow.state.value.stage)
    }
    @Test fun rejectsFutureReceiptForeignAccountAndUnverifiedIdentity() = runTest {
        val backend = Backend(); val flow = flow(backend, Journal()); flow.open(account)
        flow.review(receipt.copy(occurredOn = "2099-01-01")); assertEquals(SavingsStage.Browsing, flow.state.value.stage)
        flow.review(receipt.copy(account = AccountId(0, 3))); assertEquals(SavingsStage.Browsing, flow.state.value.stage)
        assertEquals(0, backend.calls)
        backend.identity = backend.identity.copy(emailVerified = false)
        flow.open(account); flow.review(receipt); flow.confirm(); assertEquals(0, backend.calls)
    }
    @Test fun configurationHasDateSlotIdentityAndNoGeneratedReceiptKey() = runTest {
        val backend = Backend(); val journal = Journal(); val flow = flow(backend, journal)
        flow.open(account)
        val action = SavingsAction.Terms(account.id, SavingsTermsCommand(BusinessDate(2099, 1, 1), "quarterly", false))
        flow.review(action); flow.confirm()
        assertNull(journal.pending!!.key)
        assertEquals(action, PendingSavings.decode(journal.pending!!.encode()).action)
    }
    @Test fun receiptMayCreditAnotherOwnedAccountWithoutCreditingSavings() = runTest {
        val backend = Backend(); val journal = Journal()
        val receiving = account.copy(id = AccountId(0, 3), type = AccountType.NORMAL, name = AccountName("Everyday"))
        backend.extraAccounts = listOf(receiving)
        val flow = flow(backend, journal); flow.open(account)
        flow.review(receipt.copy(account = receiving.id)); flow.confirm()
        assertEquals(receiving.id, journal.pending!!.action.account)
        assertEquals(1, backend.calls)
    }
    @Test fun rejectedConfigurationSlotCanBeCorrectedWithoutDiscardingUnknownReceipt() = runTest {
        val backend = Backend(); val journal = Journal(); val flow = flow(backend, journal)
        flow.open(account)
        flow.review(SavingsAction.Rate(account.id, SavingsRateCommand(BusinessDate(2026, 11, 10), "3")))
        backend.result = RepositoryResult.Failure(ApiFailure.Http(409, null, null, null))
        flow.confirm()
        assertNull(journal.pending)
        assertEquals(SavingsStage.Browsing, flow.state.value.stage)
        flow.review(receipt); flow.confirm()
        assertNotNull(journal.pending)
        assertEquals(SavingsStage.Uncertain, flow.state.value.stage)
    }

}
