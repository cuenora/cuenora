package org.cuenora.shared.onboarding

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import org.cuenora.shared.validation.AccountForm
import kotlin.test.*

class IndependentOnboardingProbeTest {
    private val owner = OwnerId(0, 7)
    private class Store : PendingAccountStore {
        val values = mutableMapOf<OwnerId, PendingAccount>()
        override suspend fun read(owner: OwnerId) = values[owner]
        override suspend fun save(pending: PendingAccount) { check(values[pending.owner] == null); values[pending.owner] = pending }
        override suspend fun clear(owner: OwnerId) { values.remove(owner) }
    }
    @Test fun cancelledPostConcurrentDuplicateAndRestartPreserveExactOperation() = runTest {
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val seen = mutableListOf<String>(); var cancel = true
        val backend = object : OnboardingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun requestVerification() = RepositoryResult.Success(Unit)
            override suspend fun confirmVerification(token: String) = RepositoryResult.Success(Unit)
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(true)
            override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
                seen += pending.encode(); entered.complete(Unit)
                if (cancel) release.await()
                return RepositoryResult.Success(Unit)
            }
        }
        val store = Store(); var keys = 0
        fun flow() = OnboardingSession(backend, store, { keys++; IdempotencyKey("key-$keys") }, { "2026-11-08" })
        // Seed a journal as though the previous process stopped after durable save.
        val original = PendingAccount(owner, CreateAccountCommand(AccountName("Original"), AccountType.NORMAL, Currency.SEK,
            OpeningBalance(Money(1234, Currency.SEK), BusinessDate(2026,11,8))), IdempotencyKey("original-key"))
        store.save(original)
        val first = flow(); first.load(); assertEquals(OnboardingStage.Uncertain, first.state.value.stage)
        val saving = launch { first.save() }; entered.await()
        first.cancel(); first.begin(); first.save()
        assertEquals(1, seen.size); saving.cancelAndJoin()
        assertEquals(original.encode(), store.read(owner)!!.encode())
        assertEquals(OnboardingStage.Uncertain, first.state.value.stage)
        val restarted = flow(); restarted.load(); restarted.review(AccountForm("Different", AccountType.SAVINGS, Currency.EUR, null, null))
        cancel = false; restarted.save(); restarted.save()
        assertEquals(listOf(original.encode(), original.encode()), seen)
        assertEquals(0, keys); assertNull(store.read(owner)); assertEquals(OnboardingStage.Complete, restarted.state.value.stage)
    }
    @Test fun realLogoutLoginOwnerSwitchAnd401CannotUseStaleCommandOrZone() = runTest {
        var active = "a"; var unauthorized = false; val writes = mutableListOf<String>()
        val authStore = object : SessionStore {
            var value: SessionCredentials? = null
            override suspend fun read() = value
            override suspend fun replace(credentials: SessionCredentials) { value = credentials }
            override suspend fun clear() { value = null }
        }
        val repo = object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials(active, "r-$active"), 900))
            override suspend fun refresh(credentials: SessionCredentials) = login("", "")
            override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
        }
        val engine = MockEngine { req ->
            if (req.method != HttpMethod.Get) writes += req.url.encodedPath
            if (unauthorized) respond("{}", HttpStatusCode.Unauthorized)
            else respond("""{"id":"00000000-0000-0000-0000-00000000000${if(active=="a")7 else 8}","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "https://example.invalid"); val session = AuthSession(repo, authStore) { 0L }
        val backend = AuthenticatedOnboardingBackend(session, api)
        val pending = PendingAccount(owner, CreateAccountCommand(AccountName("A"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("a-key"))
        try {
            session.login("a@example.test", "x"); assertEquals(owner, (backend.profile() as RepositoryResult.Success).value.ownerId)
            session.logout(); active = "b"; session.login("b@example.test", "x")
            assertIs<RepositoryResult.Failure>(backend.create(pending))
            assertIs<RepositoryResult.Failure>(backend.confirmZone(owner, "Europe/Paris"))
            assertTrue(writes.isEmpty()); assertEquals("b", authStore.value!!.accessToken)
            unauthorized = true; assertIs<RepositoryResult.Failure>(backend.profile())
            assertNull(authStore.value); assertEquals(AuthPhase.SignedOut, session.state.value.phase)
            unauthorized = false; assertIs<RepositoryResult.Failure>(backend.create(pending)); assertTrue(writes.isEmpty())
        } finally { api.close(); engine.close() }
    }
}
