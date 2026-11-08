package org.cuenora.shared.auth

import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthorizedSessionTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }
    @Test fun resourceRequestCannotRaceRenewalOrLogoutAndNeverUsesAnOldBearer() = runTest {
        var now = 0L
        val store = Store()
        val calls = mutableListOf<String>()
        val repository = object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) =
                RepositoryResult.Success(TokenGrant(SessionCredentials("old-access", "old-refresh"), 900))
            override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> {
                calls += "refresh"; return RepositoryResult.Success(TokenGrant(SessionCredentials("new-access", "new-refresh"), 900))
            }
            override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> {
                calls += "logout"; return RepositoryResult.Success(Unit)
            }
        }
        val session = AuthSession(repository, store) { now }
        session.login("person@example.test", "x")
        now = 900_000
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val request = async { session.authorized { token ->
            assertEquals("new-access", token); calls += "request"; entered.complete(Unit); release.await()
            RepositoryResult.Success(Unit)
        } }
        entered.await()
        val logout = async { session.logout() }
        yield(); assertEquals(listOf("refresh", "request"), calls)
        release.complete(Unit); request.await(); logout.await()
        assertEquals(listOf("refresh", "request", "logout"), calls)
        var used = false
        assertIs<RepositoryResult.Failure>(session.authorized { used = true; RepositoryResult.Success(Unit) })
        assertFalse(used); assertNull(store.value)
    }

    @Test fun unauthorizedResourceClearsSessionButNetworkFailureDoesNot() = runTest {
        val store = Store()
        val repo = object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(
                TokenGrant(SessionCredentials("access", "refresh"), 900))
            override suspend fun refresh(credentials: SessionCredentials) = login("", "")
            override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
        }
        val session = AuthSession(repo, store) { 0L }
        session.login("person@example.test", "x")
        session.authorized<Unit> { RepositoryResult.Failure(ApiFailure.Network) }
        assertEquals(AuthPhase.SignedIn, session.state.value.phase)
        session.authorized<Unit> { RepositoryResult.Failure(ApiFailure.Http(401, null, null, null)) }
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
        assertNull(store.value)
    }
}
