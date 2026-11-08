package org.cuenora.shared.recording

import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.launch
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.TransactionForm
import kotlin.test.*

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class RecordingSessionTest {
    private val owner = OwnerId(0, 1)
    private class Store : PendingTransactionStore {
        var pending: PendingTransaction? = null
        var failSave = false
        override suspend fun read(owner: OwnerId) = pending
        override suspend fun save(pending: PendingTransaction) { check(!failSave); this.pending = pending }
        override suspend fun clear(owner: OwnerId) { pending = null }
    }
    private inner class Backend : RecordingBackend {
        var posts = 0
        var items = emptyList<Account>()
        var gate: kotlinx.coroutines.CompletableDeferred<Unit>? = null
        var profileGate: kotlinx.coroutines.CompletableDeferred<Unit>? = null
        var profileFailure = false
        var result: RepositoryResult<Unit> = RepositoryResult.Success(Unit)
        var identity = OnboardingProfile(owner, true, "Europe/Stockholm")
        override suspend fun profile(): RepositoryResult<OnboardingProfile> {
            kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) { profileGate?.await() }
            return if (profileFailure) RepositoryResult.Failure(ApiFailure.Network) else RepositoryResult.Success(identity)
        }
        override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> {
            identity = identity.copy(timeZone = zone); return RepositoryResult.Success(zone)
        }
        override suspend fun accounts(owner: OwnerId) = RepositoryResult.Success(items)
        override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> { posts++; gate?.await(); return result }
    }
    private fun session(backend: Backend, store: Store) = RecordingSession(backend, store,
        { RepositoryResult.Success(emptyList()) }, { IdempotencyKey("one-operation") }, { "2026-11-08" })
    private fun form(kind: TransactionKind = TransactionKind.EXPENSE) =
        TransactionForm(kind, "12.34", Currency.SEK, "2026-11-08", description = "Synthetic")

    @Test fun incomeAndExpenseRequireReviewAndSupportNoAccount() = runTest {
        for (kind in listOf(TransactionKind.INCOME, TransactionKind.EXPENSE)) {
            val backend = Backend(); val store = Store(); val flow = session(backend, store)
            flow.begin(); flow.update(form(kind)); flow.save()
            assertEquals(0, backend.posts)
            flow.review(); assertEquals(RecordingStage.Review, flow.state.value.stage)
            flow.save(); assertEquals(1, backend.posts)
            assertEquals(RecordingStage.Saved, flow.state.value.stage); assertNull(store.pending)
        }
    }
    @Test fun invalidFutureDateAndNonPositiveAmountDoNotPost() = runTest {
        val backend = Backend(); val flow = session(backend, Store()); flow.begin()
        for (invalid in listOf(form().copy(amount = "0"), form().copy(amount = "1.234"),
            form().copy(occurredOn = "2026-11-09"), form().copy(occurredOn = "2026-02-30"))) {
            flow.update(invalid); flow.review(); flow.save()
            assertEquals(RecordingStage.Form, flow.state.value.stage); assertTrue(flow.state.value.issues.isNotEmpty())
        }
        assertEquals(0, backend.posts)
    }
    @Test fun unknownOutcomeRestoresOriginalWithoutAutomaticPost() = runTest {
        val backend = Backend(); val store = Store(); val flow = session(backend, store)
        backend.result = RepositoryResult.Failure(ApiFailure.Network)
        flow.begin(); flow.update(form()); flow.review(); flow.save()
        val original = store.pending!!.encode()
        assertEquals(RecordingStage.Uncertain, flow.state.value.stage)
        flow.update(form().copy(amount = "99")); flow.cancel()
        assertEquals(original, store.pending!!.encode())
        val restored = session(backend, store); restored.begin()
        assertEquals(1, backend.posts); assertEquals(RecordingStage.Uncertain, restored.state.value.stage)
        backend.result = RepositoryResult.Success(Unit); restored.save()
        assertEquals(2, backend.posts); assertNull(store.pending)
    }
    @Test fun storageFailurePreventsPostAndCancellationDoesNotWrite() = runTest {
        val backend = Backend(); val store = Store(); store.failSave = true
        val flow = session(backend, store); flow.begin(); flow.update(form()); flow.review(); flow.save()
        assertEquals(0, backend.posts); assertEquals(RecordingStage.Uncertain, flow.state.value.stage)
        val fresh = session(backend, Store()); fresh.begin(); fresh.update(form()); fresh.cancel()
        assertEquals(RecordingStage.Closed, fresh.state.value.stage); assertEquals(0, backend.posts)
    }
    @Test fun unverifiedOrUnconfirmedZoneBlocksNewRecording() = runTest {
        val backend = Backend(); backend.identity = OnboardingProfile(owner, false, null)
        val flow = session(backend, Store()); flow.begin()
        assertEquals(RecordingStage.Blocked, flow.state.value.stage)
        flow.update(form()); flow.review(); flow.save(); assertEquals(0, backend.posts)
    }
    @Test fun explicitZoneConfirmationUnblocksAccountlessRecording() = runTest {
        val backend = Backend(); backend.identity = OnboardingProfile(owner, true, null)
        val flow = session(backend, Store()); flow.begin()
        assertEquals(RecordingStage.Blocked, flow.state.value.stage)
        flow.confirmZone("Europe/Stockholm"); assertEquals(RecordingStage.Form, flow.state.value.stage)
        assertEquals(0, backend.posts)
    }
    @Test fun assignedAccountMustMatchCurrencyStatusAndIncomePolicy() = runTest {
        val backend = Backend()
        val account = Account(AccountId(0, 9), owner, AccountName("Savings"), AccountType.SAVINGS, Currency.EUR, AccountStatus.ACTIVE)
        backend.items = listOf(account)
        val flow = session(backend, Store()); flow.begin(account)
        assertEquals(account.id, flow.state.value.form.accountId); assertEquals(Currency.EUR, flow.state.value.form.currency)
        for (invalid in listOf(form().copy(accountId = account.id),
                form(TransactionKind.INCOME).copy(currency = Currency.EUR, accountId = account.id))) {
            flow.update(invalid); flow.review(); assertEquals(RecordingStage.Form, flow.state.value.stage)
            assertTrue(flow.state.value.issues.any { it.field == "accountId" })
        }
        flow.update(form().copy(currency = Currency.EUR, accountId = account.id)); flow.review()
        assertEquals(RecordingStage.Review, flow.state.value.stage)
    }
    @Test fun duplicateSubmitAndRetiredCompletionCannotPublishOrClearJournal() = runTest {
        val backend = Backend(); backend.gate = kotlinx.coroutines.CompletableDeferred()
        val store = Store(); val flow = session(backend, store)
        flow.begin(); flow.update(form()); flow.review()
        val job = launch { flow.save() }
        runCurrent()
        flow.save(); assertEquals(1, backend.posts)
        flow.clear(); backend.gate!!.complete(Unit); job.join()
        assertEquals(RecordingStage.Closed, flow.state.value.stage); assertNotNull(store.pending)
    }
    @Test fun explicitValidationRejectionAllowsCorrectionWithoutAssumedSuccess() = runTest {
        val backend = Backend(); backend.result = RepositoryResult.Failure(ApiFailure.Http(400, null, null, null))
        val store = Store(); val flow = session(backend, store)
        flow.begin(); flow.update(form()); flow.review(); flow.save()
        assertEquals(RecordingStage.Form, flow.state.value.stage); assertNull(store.pending)
        assertEquals("12.34", flow.state.value.form.amount); assertEquals(1, backend.posts)
    }

    @Test fun retiredSessionRejectsNonCooperativeFailedProfileCompletion() = runTest {
        val backend = Backend()
        backend.profileFailure = true
        backend.profileGate = kotlinx.coroutines.CompletableDeferred()
        val flow = session(backend, Store())
        val job = launch { flow.begin() }
        runCurrent()
        flow.clear()
        backend.profileGate!!.complete(Unit)
        job.join()
        assertEquals(RecordingStage.Closed, flow.state.value.stage)
    }

}
