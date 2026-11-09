package org.cuenora.shared.onboarding

import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import kotlin.test.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.AccountForm

class AdditionalAccountIndependentTest {
    private val owner = OwnerId(0, 7)
    private val form = AccountForm("Additional", AccountType.SAVINGS, Currency.EUR, "-12.34", "2026-11-09")
    private class Backend : OnboardingBackend {
        var verified = true
        var existing = true
        val trace = mutableListOf<String>()
        val writes = mutableListOf<PendingAccount>()
        var beforeProfile: suspend () -> Unit = {}
        var beforeCreate: suspend () -> Unit = {}
        var result: RepositoryResult<Unit> = RepositoryResult.Success(Unit)
        override suspend fun profile(): RepositoryResult<OnboardingProfile> {
            trace += "profile"
            beforeProfile()
            return RepositoryResult.Success(OnboardingProfile(OwnerId(0, 7), verified, "Europe/Stockholm"))
        }
        override suspend fun requestVerification() = RepositoryResult.Success(Unit)
        override suspend fun confirmVerification(token: String): RepositoryResult<Unit> {
            verified = true
            return RepositoryResult.Success(Unit)
        }
        override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
        override suspend fun hasAccounts(owner: OwnerId): RepositoryResult<Boolean> {
            trace += "accounts"
            return RepositoryResult.Success(existing)
        }
        override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
            writes += pending
            beforeCreate()
            return result
        }
    }
    private class Store(private val trace: MutableList<String>) : PendingAccountStore {
        var record: PendingAccount? = null
        var beforeRead: suspend () -> Unit = {}
        var afterSave: suspend () -> Unit = {}
        override suspend fun read(owner: OwnerId): PendingAccount? {
            trace += "journal"
            beforeRead()
            return record?.takeIf { it.owner == owner }
        }
        override suspend fun save(pending: PendingAccount) { record = pending; afterSave() }
        override suspend fun clear(owner: OwnerId) { record = null }
    }
    private fun flow(backend: Backend, store: Store, newKey: () -> IdempotencyKey = { IdempotencyKey("fresh") }) =
        OnboardingSession(backend, store, newKey, { "2026-11-09" })

    @Test fun intentRefreshesVerificationThenRestoresOriginalJournalWithoutPosting() = runTest {
        val backend = Backend().apply { verified = false }
        val store = Store(backend.trace)
        val original = PendingAccount(owner, CreateAccountCommand(AccountName("Original"), AccountType.NORMAL, Currency.SEK),
            IdempotencyKey("original"))
        store.record = original
        var keys = 0
        val flow = flow(backend, store) { keys++; IdempotencyKey("replacement") }
        flow.beginAdditional()
        assertEquals(OnboardingStage.Verify, flow.state.value.stage)
        assertEquals(listOf("profile"), backend.trace)
        flow.confirmVerification("ev1_" + "a".repeat(64))
        assertEquals(listOf("profile", "profile", "journal"), backend.trace)
        assertEquals(OnboardingStage.Uncertain, flow.state.value.stage)
        flow.cancel(); flow.begin(); flow.review(form); flow.beginAdditional()
        assertTrue(backend.writes.isEmpty())
        assertEquals(original, store.record)
        flow.save()
        assertEquals(listOf(original), backend.writes)
        assertEquals(0, keys)
        assertEquals(OnboardingStage.Complete, flow.state.value.stage)
    }

    @Test fun completeAndExistingPermitNewIntentButFirstAccountDefaultRemains() = runTest {
        val backend = Backend()
        val store = Store(backend.trace)
        var keys = 0
        val flow = flow(backend, store) { IdempotencyKey("key-${++keys}") }
        flow.load(); flow.begin()
        assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        flow.beginAdditional(); flow.cancel()
        assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        assertTrue(backend.writes.isEmpty())
        repeat(2) {
            flow.beginAdditional(); flow.review(form); flow.save()
            assertEquals(OnboardingStage.Complete, flow.state.value.stage)
        }
        assertEquals(2, backend.writes.map { it.key }.distinct().size)
        assertEquals(Money(-1234, Currency.EUR), backend.writes[0].command.openingBalance!!.amount)
        backend.existing = false
        flow.leaveAdditional(); flow.load()
        assertEquals(OnboardingStage.Empty, flow.state.value.stage)
        flow.begin(); flow.cancel()
        assertEquals(OnboardingStage.Empty, flow.state.value.stage)
    }

    @Test fun retirementAtProfileJournalReadSaveAndCreateCannotPublishOrPostLater() = runTest {
        for (boundary in listOf("profile", "journal", "save", "create")) for (failure in listOf(false, true)) {
            val backend = Backend()
            val store = Store(backend.trace)
            val entered = CompletableDeferred<Unit>()
            val release = CompletableDeferred<Unit>()
            val flow = flow(backend, store)
            val gate: suspend () -> Unit = {
                entered.complete(Unit)
                release.await()
                if (failure) throw CancellationException("synthetic abandoned request")
            }
            when (boundary) {
                "profile" -> backend.beforeProfile = gate
                "journal" -> store.beforeRead = gate
                "save" -> store.afterSave = gate
                "create" -> backend.beforeCreate = gate
            }
            val job = launch {
                if (boundary in listOf("profile", "journal")) flow.beginAdditional()
                else { flow.beginAdditional(); flow.review(form); flow.save() }
            }
            entered.await(); flow.clear(); release.complete(Unit); job.join()
            assertEquals(OnboardingState(), flow.state.value, "$boundary failure=$failure")
            assertEquals(if (boundary == "create") 1 else 0, backend.writes.size)
            flow.beginAdditional(); flow.load(); flow.begin(); flow.save()
            assertEquals(if (boundary == "create") 1 else 0, backend.writes.size)
            if (boundary in listOf("save", "create")) assertNotNull(store.record)
        }
    }

    @Test fun unknownCommandCannotBeReplacedOrAutomaticallyReplayed() = runTest {
        val backend = Backend()
        val store = Store(backend.trace)
        var keys = 0
        val flow = flow(backend, store) { IdempotencyKey("key-${++keys}") }
        flow.beginAdditional(); flow.review(form)
        backend.result = RepositoryResult.Failure(ApiFailure.Timeout)
        flow.save()
        val original = assertNotNull(store.record)
        flow.leaveAdditional(); flow.beginAdditional(); flow.review(form.copy(name = "Replacement")); flow.begin()
        assertEquals(OnboardingStage.Uncertain, flow.state.value.stage)
        assertEquals(1, backend.writes.size)
        assertEquals(1, keys)
        backend.result = RepositoryResult.Success(Unit)
        flow.save()
        assertEquals(listOf(original, original), backend.writes)
        assertNull(store.record)
    }
}
