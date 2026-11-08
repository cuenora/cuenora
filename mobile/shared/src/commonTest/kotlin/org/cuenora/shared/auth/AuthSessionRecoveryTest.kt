package org.cuenora.shared.auth

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import kotlin.test.*

/** Regression coverage retained from independent security/concurrency verification. */
class AuthSessionRecoveryTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        var replaceThenFail = false
        var replaceThenCancel = false
        var clearFails = false
        var writes = 0

        override suspend fun read() = value
        override suspend fun clear() {
            if (clearFails) throw SessionStoreException(SessionFailure.Unavailable)
            value = null
            writes++
        }
        override suspend fun replace(credentials: SessionCredentials) {
            value = credentials
            writes++
            if (replaceThenFail) throw SessionStoreException(SessionFailure.Unavailable)
            if (replaceThenCancel) throw CancellationException()
        }
    }

    private class Repository : AuthRepository {
        var calls = 0
        var refreshes = 0
        var logoutFailure = false
        var refreshHook: suspend () -> Unit = {}
        var revoked: String? = null
        private fun grant() = RepositoryResult.Success(
            TokenGrant(SessionCredentials("access-$calls", "refresh-$calls"), 60),
        )
        override suspend fun register(email: String, password: String): RepositoryResult<Unit> {
            calls++
            return RepositoryResult.Success(Unit)
        }
        override suspend fun login(email: String, password: String): RepositoryResult<TokenGrant> {
            calls++
            return grant()
        }
        override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> {
            calls++
            refreshes++
            refreshHook()
            return grant()
        }
        override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> {
            revoked = credentials.refreshToken
            return if (logoutFailure) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(Unit)
        }
    }

    @Test fun invalidFormsCannotMutatePersistenceOrCallNetwork() = runTest {
        val store = Store()
        val repository = Repository()
        val session = AuthSession(repository, store) { 0 }
        session.restore()
        val before = store.writes
        session.login("bad", "x")
        session.register("bad", "x")
        assertEquals(before, store.writes)
        assertNull(store.value)
        assertEquals(0, repository.calls)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
        assertTrue(session.state.value.issues.isNotEmpty())
    }

    @Test fun ambiguousWriteNeverClaimsSuccessAndRestartUsesNewPairOnly() = runTest {
        val store = Store().apply { replaceThenFail = true }
        val repository = Repository()
        val session = AuthSession(repository, store) { 0 }
        session.login("x@example.test", "x")
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        assertEquals("refresh-1", store.value?.refreshToken)
        store.replaceThenFail = false
        AuthSession(repository, store) { 0 }.restore()
        assertEquals(1, repository.refreshes)
        assertEquals("refresh-2", store.value?.refreshToken)
    }

    @Test fun cancellationAfterCommittedWriteClearsPairAndCannotRestoreIt() = runTest {
        val store = Store().apply { replaceThenCancel = true }
        val repository = Repository()
        val session = AuthSession(repository, store) { 0 }
        assertFailsWith<CancellationException> { session.login("x@example.test", "x") }
        assertNull(store.value)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
        store.replaceThenCancel = false
        AuthSession(repository, store) { 0 }.restore()
        assertEquals(0, repository.refreshes)
    }

    @Test fun refreshQueueThenLogoutCannotResurrectAndFailedRevocationIsVisible() = runTest {
        var now = 0L
        val store = Store()
        val repository = Repository()
        val session = AuthSession(repository, store) { now }
        session.login("x@example.test", "x")
        now = 60_000
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        repository.refreshHook = {
            assertNull(store.value)
            entered.complete(Unit)
            release.await()
        }
        val first = async { session.renewIfNeeded() }
        entered.await()
        val second = async { session.renewIfNeeded() }
        repository.logoutFailure = true
        val logout = async { session.logout() }
        release.complete(Unit)
        awaitAll(first, second, logout)
        assertEquals(1, repository.refreshes)
        assertEquals("refresh-2", repository.revoked)
        assertNull(store.value)
        assertEquals(AuthNotice.RemoteLogoutUnconfirmed, session.state.value.notice)
        AuthSession(repository, store) { now }.restore()
        assertEquals(1, repository.refreshes)
    }

    @Test fun failedLocalRemovalBlocksRefreshAndLogoutWithoutRemoteEffects() = runTest {
        var now = 0L
        val store = Store()
        val repository = Repository()
        val session = AuthSession(repository, store) { now }
        session.login("x@example.test", "x")
        store.clearFails = true
        now = 60_000
        session.renewIfNeeded()
        session.logout()
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        assertEquals("refresh-1", store.value?.refreshToken)
        assertEquals(0, repository.refreshes)
        assertNull(repository.revoked)
    }

    @Test fun serverPayloadDoesNotReachPresentationAndRegistrationCreatesNoSession() = runTest {
        var calls = 0
        val engine = MockEngine {
            calls++
            if (it.url.encodedPath.endsWith("register")) {
                respond("""{"status":"accepted"}""", HttpStatusCode.Accepted)
            } else {
                respond("""{"error":{"code":"invalid_credentials","message":"synthetic-private-payload"}}""",
                    HttpStatusCode.Unauthorized)
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val store = Store()
            val session = AuthSession(HttpAuthRepository(api), store) { 0 }
            session.register("x@example.test", "synthetic strong password")
            assertEquals(AuthNotice.RegistrationAccepted, session.state.value.notice)
            assertNull(store.value)
            session.login("x@example.test", "x")
            assertEquals(AuthNotice.InvalidCredentials, session.state.value.notice)
            assertFalse(session.state.value.toString().contains("synthetic-private-payload"))
            assertEquals(2, calls)
        } finally { api.close(); engine.close() }
    }
}
