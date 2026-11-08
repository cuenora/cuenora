package org.cuenora.shared.onboarding

import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.AccountForm
import kotlin.test.*

class OnboardingSessionTest {
    private val owner = OwnerId(0, 7)
    private class Backend : OnboardingBackend {
        var profileValue = OnboardingProfile(OwnerId(0, 7), false, null)
        var existing = false
        var creates = 0
        var keys = mutableListOf<IdempotencyKey>()
        var next: RepositoryResult<Unit> = RepositoryResult.Success(Unit)
        override suspend fun profile() = RepositoryResult.Success(profileValue)
        override suspend fun requestVerification() = RepositoryResult.Success(Unit)
        override suspend fun confirmVerification(token: String): RepositoryResult<Unit> {
            profileValue = profileValue.copy(emailVerified = true); return RepositoryResult.Success(Unit)
        }
        override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> {
            profileValue = profileValue.copy(timeZone = zone); return RepositoryResult.Success(zone)
        }
        override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(existing)
        override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
            creates++; keys += pending.key; return next
        }
    }
    private class Store : PendingAccountStore {
        var value: PendingAccount? = null
        var fail = false
        override suspend fun read(owner: OwnerId) = value?.takeIf { it.owner == owner }
        override suspend fun save(pending: PendingAccount) { if (fail) throw PendingAccountStoreException(); value = pending }
        override suspend fun clear(owner: OwnerId) { if (fail) throw PendingAccountStoreException(); value = null }
    }
    private fun coordinator(backend: Backend, store: Store) = OnboardingSession(backend, store,
        { IdempotencyKey("synthetic-key") }, { "2026-11-08" })
    private fun form() = AccountForm("Daily", AccountType.NORMAL, Currency.SEK, "12.34", "2026-11-08")

    @Test fun verificationAcceptanceDoesNotAdvanceAndConfirmedProfileZoneIsPreserved() = runTest {
        val backend = Backend(); val store = Store(); val flow = coordinator(backend, store)
        flow.load(); assertEquals(OnboardingStage.Verify, flow.state.value.stage)
        flow.requestVerification(); assertEquals(OnboardingStage.Verify, flow.state.value.stage)
        assertEquals(OnboardingNotice.MailAccepted, flow.state.value.notice)
        flow.confirmVerification("ev1_" + "a".repeat(64))
        assertEquals(OnboardingStage.Empty, flow.state.value.stage)
        flow.begin(); flow.review(form())
        assertEquals(OnboardingStage.Form, flow.state.value.stage)
        assertEquals(0, backend.creates)
        flow.confirmZone("Europe/Stockholm")
        flow.review(form()); assertEquals(OnboardingStage.Review, flow.state.value.stage)
        assertNull(store.value)
        flow.save(); assertEquals(OnboardingStage.Complete, flow.state.value.stage)
        assertEquals(1, backend.creates)
        assertNull(store.value)
        flow.save(); assertEquals(1, backend.creates)
    }

    @Test fun uncertainWriteRestoresOriginalCommandAndKeyEvenIfAccountAlreadyAppears() = runTest {
        val backend = Backend().apply { profileValue = profileValue.copy(emailVerified = true, timeZone = "Europe/Stockholm") }
        val store = Store(); val flow = coordinator(backend, store)
        flow.load(); flow.begin(); flow.review(form())
        backend.next = RepositoryResult.Failure(ApiFailure.Timeout)
        flow.save()
        val original = assertNotNull(store.value)
        assertEquals(OnboardingStage.Uncertain, flow.state.value.stage)
        backend.existing = true
        val restarted = coordinator(backend, store); restarted.load()
        assertEquals(OnboardingStage.Uncertain, restarted.state.value.stage)
        restarted.begin(); restarted.review(form().copy(name = "Different"))
        backend.next = RepositoryResult.Success(Unit); restarted.save()
        assertEquals(listOf(original.key, original.key), backend.keys)
        assertEquals(OnboardingStage.Complete, restarted.state.value.stage)
    }

    @Test fun invalidFormCancellationAndUnavailableJournalCannotCreateAnything() = runTest {
        val backend = Backend().apply { profileValue = profileValue.copy(emailVerified = true, timeZone = "Europe/Stockholm") }
        val store = Store(); val flow = coordinator(backend, store)
        flow.load(); flow.begin(); flow.review(form().copy(openingBalanceAmount = "1.234"))
        assertEquals(OnboardingStage.Form, flow.state.value.stage)
        flow.cancel(); assertEquals(OnboardingStage.Empty, flow.state.value.stage)
        flow.begin(); flow.review(form()); store.fail = true; flow.save()
        assertEquals(0, backend.creates)
        assertEquals(OnboardingNotice.StorageUnavailable, flow.state.value.notice)
    }

    @Test fun ownerScopedPendingCodecRoundTripsExactOpeningAndDoesNotPrintData() {
        val pending = PendingAccount(owner, CreateAccountCommand(AccountName("Daily"), AccountType.SAVINGS,
            Currency.USD, OpeningBalance(Money(-123, Currency.USD), BusinessDate(2026, 2, 28))), IdempotencyKey("stable-key"))
        val decoded = PendingAccount.decode(pending.encode())
        assertEquals(owner, decoded.owner)
        assertEquals(pending.command, decoded.command)
        assertEquals(pending.key, decoded.key)
        assertFalse(decoded.toString().contains("Daily"))
    }
}
