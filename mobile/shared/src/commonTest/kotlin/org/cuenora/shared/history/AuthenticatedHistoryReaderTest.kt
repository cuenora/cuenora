package org.cuenora.shared.history

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthenticatedHistoryReaderTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }
    private fun session(store: Store = Store()) = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(
            TokenGrant(SessionCredentials("synthetic-access", "synthetic-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, store) { 0L }
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":null}"""
    private val page = """{"transactions":[],"next_cursor":"next"}"""

    @Test fun readsVerifiedServerOwnerAndPassesCursorWithCurrentBearer() = runTest {
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            assertEquals("GET", request.method.value)
            assertEquals("Bearer synthetic-access", request.headers[HttpHeaders.Authorization])
            paths += request.url.encodedPath
            if (request.url.encodedPath == "/v1/transactions") {
                assertEquals("20", request.url.parameters["limit"])
                assertEquals("opaque cursor", request.url.parameters["cursor"])
            }
            respond(if (request.url.encodedPath == "/v1/me") profile else page,
                HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val result = assertIs<RepositoryResult.Success<Page<TransactionRecord>>>(
                AuthenticatedHistoryReader(auth, api).list(TransactionQuery(20, "opaque cursor")))
            assertEquals(listOf("/v1/me", "/v1/transactions"), paths)
            assertTrue(result.value.items.isEmpty())
            assertEquals("next", result.value.nextCursor)
        } finally { api.close(); engine.close() }
    }

    @Test fun deniedProfileAndExpiredSessionNeverFetchHistory() = runTest {
        for (status in listOf(200, 401, 403)) {
            var calls = 0
            val engine = MockEngine { request ->
                calls++
                assertEquals("/v1/me", request.url.encodedPath)
                respond(if (status == 200) profile.replace("true", "false") else "{}",
                    HttpStatusCode.fromValue(status), headersOf(HttpHeaders.ContentType, "application/json"))
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
                assertIs<RepositoryResult.Failure>(AuthenticatedHistoryReader(auth, api).list())
                assertEquals(1, calls)
                assertEquals(if (status == 401) AuthPhase.SignedOut else AuthPhase.SignedIn, auth.state.value.phase)
                if (status == 401) assertNull(store.value)
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun signedOutReaderDoesNotUseTransportAndNetworkFailureDoesNotBecomeEmpty() = runTest {
        var calls = 0
        val engine = MockEngine { calls++; throw kotlinx.io.IOException("synthetic offline") }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); val reader = AuthenticatedHistoryReader(auth, api)
            assertIs<RepositoryResult.Failure>(reader.list()); assertEquals(0, calls)
            auth.login("person@example.test", "x")
            assertEquals(ApiFailure.Network, assertIs<RepositoryResult.Failure>(reader.list()).error)
            assertEquals(1, calls); assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }
    @Test fun readerCannotMixPagesAcrossOwnersAndResource401ExpiresSession() = runTest {
        var otherOwner = false
        var denyResource = false
        var resourceCalls = 0
        val engine = MockEngine { request ->
            val isProfile = request.url.encodedPath == "/v1/me"
            if (!isProfile) resourceCalls++
            respond(if (isProfile) {
                if (otherOwner) profile.replace("000007", "000008") else profile
            } else if (denyResource) "{}" else page,
                if (!isProfile && denyResource) HttpStatusCode.Unauthorized else HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedHistoryReader(auth, api)
            assertIs<RepositoryResult.Success<*>>(reader.list())
            otherOwner = true
            assertEquals(403, (assertIs<RepositoryResult.Failure>(
                reader.list(TransactionQuery(cursor = "next"))).error as ApiFailure.Http).status)
            assertEquals(1, resourceCalls)
            otherOwner = false; denyResource = true
            assertIs<RepositoryResult.Failure>(reader.list())
            assertEquals(AuthPhase.SignedOut, auth.state.value.phase)
            assertEquals(2, resourceCalls)
        } finally { api.close(); engine.close() }
    }

    @Test fun independentAllResourceDenialsPreserveOrRetireSession() = runTest {
        for (resource in listOf("history", "accounts", "categories")) for (status in listOf(401, 403)) {
            val calls = mutableListOf<String>()
            val engine = MockEngine { request ->
                assertEquals("GET", request.method.value)
                calls += request.url.encodedPath
                respond(if (request.url.encodedPath == "/v1/me") profile else "{}",
                    if (request.url.encodedPath == "/v1/me") HttpStatusCode.OK else HttpStatusCode.fromValue(status),
                    headersOf(HttpHeaders.ContentType, "application/json"))
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
                val reader = AuthenticatedHistoryReader(auth, api)
                val result = when(resource) { "history" -> reader.list(); "accounts" -> reader.accounts(AccountListQuery()); else -> reader.categories() }
                assertEquals(status, (assertIs<RepositoryResult.Failure>(result).error as ApiFailure.Http).status)
                assertEquals(2, calls.size)
                assertEquals(if(status == 401) AuthPhase.SignedOut else AuthPhase.SignedIn, auth.state.value.phase)
                assertEquals(status == 401, store.value == null)
            } finally { api.close(); engine.close() }
        }
    }
    @Test fun independentOwnerSwitchBlocksAllSubsequentResourceReadsWithoutLogout() = runTest {
        var switched = false
        var reads = 0
        val engine = MockEngine { request ->
            assertEquals("GET", request.method.value)
            val profileRead = request.url.encodedPath == "/v1/me"
            if (!profileRead) reads++
            respond(if (profileRead) { if(switched) profile.replace("000007", "000008") else profile } else page,
                HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
            val originalCredentials = store.value
            val reader = AuthenticatedHistoryReader(auth, api)
            assertIs<RepositoryResult.Success<*>>(reader.list())
            switched = true
            val results = listOf(reader.list(TransactionQuery(cursor="next")), reader.accounts(AccountListQuery()), reader.categories())
            results.forEach { assertEquals(403, (assertIs<RepositoryResult.Failure>(it).error as ApiFailure.Http).status) }
            assertEquals(1, reads); assertEquals(originalCredentials, store.value)
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }
}
