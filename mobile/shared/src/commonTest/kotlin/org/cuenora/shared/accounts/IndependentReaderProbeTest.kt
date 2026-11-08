package org.cuenora.shared.accounts

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

/** Retained independent probes; request denial must not mutate credentials or bypass ordering. */
class IndependentReaderProbeTest {
    private class Store(private val events: MutableList<String>? = null) : SessionStore {
        var saved: SessionCredentials? = null
        var writes = 0
        override suspend fun read() = saved
        override suspend fun replace(credentials: SessionCredentials) { writes++; saved = credentials }
        override suspend fun clear() { events?.add("clear"); writes++; saved = null }
    }
    private fun session(store: Store, now: () -> Long, events: MutableList<String>) = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(
            TokenGrant(SessionCredentials("old", "refresh"), 60))
        override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> {
            events += "renew"
            return RepositoryResult.Success(TokenGrant(SessionCredentials("new", "new-refresh"), 900))
        }
        override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> {
            events += "logout"; return RepositoryResult.Success(Unit)
        }
    }, store, now)
    private fun profile(owner: Int = 7) =
        """{"id":"00000000-0000-0000-0000-00000000000$owner","email_verified":true,"time_zone":null}"""
    private val empty = """{"accounts":[],"next_cursor":null}"""

    @Test fun renewalAndSuspendedReadRemainOrderedBeforeLogout() = runTest {
        var now = 0L
        val events = mutableListOf<String>(); val store = Store(events)
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val auth = session(store, { now }, events)
        val engine = MockEngine { request ->
            assertEquals("Bearer new", request.headers[HttpHeaders.Authorization])
            events += request.url.encodedPath
            if (request.url.encodedPath == "/v1/accounts") { entered.complete(Unit); release.await() }
            respond(if (request.url.encodedPath == "/v1/me") profile() else empty,
                HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            auth.login("person@example.test", "x"); now = 31_000
            val reader = AuthenticatedAccountReader(auth, api)
            val read = async { reader.list() }; entered.await()
            val logout = launch { auth.logout() }; yield()
            assertFalse(logout.isCompleted); assertEquals("new", store.saved?.accessToken)
            release.complete(Unit); assertIs<RepositoryResult.Success<*>>(read.await()); logout.join()
            assertNull(store.saved); assertEquals(AuthPhase.SignedOut, auth.state.value.phase)
            assertEquals(listOf("clear", "renew", "/v1/me", "/v1/accounts", "clear", "logout"), events)
            val count = events.size
            assertIs<RepositoryResult.Failure>(reader.list())
            assertEquals(count, events.size); assertNull(store.saved)
        } finally { api.close(); engine.close() }
    }

    @Test fun ownerSwitchDenialDoesNotFetchOrMutateCredentials() = runTest {
        val store = Store(); var other = false; var fetches = 0
        val auth = session(store, { 0L }, mutableListOf())
        val engine = MockEngine { request ->
            val isProfile = request.url.encodedPath == "/v1/me"
            if (!isProfile) fetches++
            respond(if (isProfile) profile(if (other) 8 else 7) else empty,
                HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            auth.login("person@example.test", "x")
            val reader = AuthenticatedAccountReader(auth, api)
            assertIs<RepositoryResult.Success<*>>(reader.list())
            val before = store.saved; val written = store.writes
            other = true
            val result = assertIs<RepositoryResult.Failure>(reader.list(AccountListQuery(cursor = "next")))
            assertEquals(403, (result.error as org.cuenora.shared.network.ApiFailure.Http).status)
            assertEquals(1, fetches); assertEquals(before, store.saved)
            assertEquals(written, store.writes); assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }
}
