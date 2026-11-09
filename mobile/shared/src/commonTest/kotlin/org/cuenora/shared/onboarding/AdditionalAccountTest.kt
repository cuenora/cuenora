package org.cuenora.shared.onboarding

import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.AccountForm
import kotlin.test.*

class AdditionalAccountTest {
    private val owner = OwnerId(0, 7)
    private class Backend : OnboardingBackend {
        var profileValue = OnboardingProfile(OwnerId(0, 7), true, "Europe/Stockholm")
        var existing = true
        var next: RepositoryResult<Unit> = RepositoryResult.Success(Unit)
        val commands = mutableListOf<PendingAccount>()
        var delayedProfile: CompletableDeferred<RepositoryResult<OnboardingProfile>>? = null
        override suspend fun profile() = delayedProfile?.await() ?: RepositoryResult.Success(profileValue)
        override suspend fun requestVerification() = RepositoryResult.Success(Unit)
        override suspend fun confirmVerification(token: String): RepositoryResult<Unit> {
            profileValue = profileValue.copy(emailVerified = true)
            return RepositoryResult.Success(Unit)
        }
        override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
        override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(existing)
        override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
            commands += pending
            if (next is RepositoryResult.Success) existing = true
            return next
        }
    }
    private class Store : PendingAccountStore {
        var record: PendingAccount? = null
        var saved: CompletableDeferred<Unit>? = null
        var release: CompletableDeferred<Unit>? = null
        override suspend fun read(owner: OwnerId) = record?.takeIf { it.owner == owner }
        override suspend fun save(pending: PendingAccount) {
            record = pending; saved?.complete(Unit); release?.await()
        }
        override suspend fun clear(owner: OwnerId) { record = null }
    }
    private fun flow(backend: Backend, store: Store): OnboardingSession {
        var key = 0
        return OnboardingSession(backend, store, { IdempotencyKey("additional-${++key}") }, { "2026-11-09" })
    }
    private fun form() = AccountForm("Savings", AccountType.SAVINGS, Currency.EUR, "-12.34", "2026-11-09")

    @Test fun existingOwnerCanCreateAnotherReviewedAccountWithFreshOperationKey() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store)
        flow.load(); assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        flow.begin(); assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        flow.beginAdditional(); assertEquals(OnboardingStage.Form, flow.state.value.stage)
        assertTrue(backend.commands.isEmpty())
        flow.review(form()); assertEquals(OnboardingStage.Review, flow.state.value.stage)
        flow.save(); assertEquals(OnboardingStage.Complete, flow.state.value.stage)
        assertEquals(Money(-1234, Currency.EUR), backend.commands.single().command.openingBalance?.amount)
        flow.beginAdditional(); flow.review(form().copy(openingBalanceAmount = null, openingBalanceOccurredOn = null)); flow.save()
        assertEquals(2, backend.commands.size)
        assertNotEquals(backend.commands[0].key, backend.commands[1].key)
        assertNull(store.record)
    }
    @Test fun cancelReturnsToExistingAndUncertainCreationCannotBeReplaced() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store)
        flow.beginAdditional(); flow.cancel()
        assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        flow.beginAdditional(); flow.review(form()); backend.next = RepositoryResult.Failure(ApiFailure.Timeout); flow.save()
        val original = assertNotNull(store.record)
        flow.beginAdditional(); assertEquals(OnboardingStage.Uncertain, flow.state.value.stage)
        flow.begin(); flow.review(form().copy(name = "Replacement"))
        assertEquals(1, backend.commands.size)
        backend.next = RepositoryResult.Success(Unit); flow.save()
        assertEquals(listOf(original.key, original.key), backend.commands.map { it.key })
    }
    @Test fun additionalIntentSurvivesVerificationAndFirstAccountDefaultStaysUnchanged() = runTest {
        val backend = Backend().apply { profileValue = profileValue.copy(emailVerified = false) }
        val store = Store(); val flow = flow(backend, store)
        flow.beginAdditional(); assertEquals(OnboardingStage.Verify, flow.state.value.stage)
        flow.confirmVerification("ev1_" + "a".repeat(64))
        assertEquals(OnboardingStage.Form, flow.state.value.stage)
        flow.cancel(); assertEquals(OnboardingStage.Existing, flow.state.value.stage)
        backend.existing = false; flow.load(); assertEquals(OnboardingStage.Empty, flow.state.value.stage)
        flow.begin(); flow.cancel(); assertEquals(OnboardingStage.Empty, flow.state.value.stage)
        assertTrue(backend.commands.isEmpty())
    }
    @Test fun retirementDropsLateProfilesAndPreventsPostAfterJournalSave() = runTest {
        for (failure in listOf(false, true)) {
            val backend = Backend().apply { delayedProfile = CompletableDeferred() }
            val flow = flow(backend, Store())
            val job = launch(start = CoroutineStart.UNDISPATCHED) { flow.beginAdditional() }
            flow.clear()
            backend.delayedProfile!!.complete(if (failure) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(backend.profileValue))
            job.join(); assertEquals(OnboardingState(), flow.state.value)
            assertTrue(backend.commands.isEmpty())
        }
        val backend = Backend(); val store = Store().apply { saved = CompletableDeferred(); release = CompletableDeferred() }
        val flow = flow(backend, store); flow.beginAdditional(); flow.review(form())
        val job = launch { flow.save() }; store.saved!!.await()
        flow.clear(); store.release!!.complete(Unit); job.join()
        assertTrue(backend.commands.isEmpty()); assertNotNull(store.record)
        assertEquals(OnboardingState(), flow.state.value)
    }
}
