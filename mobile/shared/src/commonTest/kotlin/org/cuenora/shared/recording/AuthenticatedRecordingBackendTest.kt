package org.cuenora.shared.recording

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthenticatedRecordingBackendTest {
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""

    @Test fun accountReadFullyPaginatesUsingCurrentAuthenticatedOwner() = runTest {
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath + (request.url.parameters["cursor"]?.let { "?cursor=$it" } ?: "")
            val body = when {
                request.url.encodedPath == "/v1/me" -> profile
                request.url.parameters["cursor"] == null -> """{"accounts":[${account("9")}],"next_cursor":"next"}"""
                else -> """{"accounts":[${account("a")}],"next_cursor":null}"""
            }
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val backend = AuthenticatedRecordingBackend(session(), api)
            val accounts = assertIs<RepositoryResult.Success<List<Account>>>(backend.accounts(OwnerId(0, 7))).value
            assertEquals(2, accounts.size)
            assertTrue(accounts.all { it.ownerId == OwnerId(0, 7) })
            assertEquals(listOf("/v1/me", "/v1/accounts", "/v1/accounts?cursor=next"), paths)
        } finally { api.close(); engine.close() }
    }

    @Test fun unconfirmedProfileBlocksAccountReads() = runTest {
        var accountReads = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/accounts") accountReads++
            respond(if (request.url.encodedPath == "/v1/me") profile.replace("\"Europe/Stockholm\"", "null") else "{}",
                HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            assertIs<RepositoryResult.Failure>(AuthenticatedRecordingBackend(session(), api).accounts(OwnerId(0, 7)))
            assertEquals(0, accountReads)
        } finally { api.close(); engine.close() }
    }

    @Test fun repeatedPaginationCursorFailsInsteadOfReturningAnIncompleteList() = runTest {
        var accountReads = 0
        val engine = MockEngine { request ->
            val body = if (request.url.encodedPath == "/v1/me") profile else {
                accountReads++
                """{"accounts":[],"next_cursor":"same"}"""
            }
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val result = AuthenticatedRecordingBackend(session(), api).accounts(OwnerId(0, 7))
            assertEquals(200, (assertIs<RepositoryResult.Failure>(result).error as ApiFailure.InvalidResponse).status)
            assertEquals(2, accountReads)
        } finally { api.close(); engine.close() }
    }

    @Test fun uncertainReplayReachesServerWithOriginalKeyWithoutRecheckingArchivedAccount() = runTest {
        var accountReads = 0
        var postKey: String? = null
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond(profile, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/accounts" -> {
                    accountReads++
                    respond("""{"accounts":[${account("9").replace("\"active\"", "\"archived\"")}],"next_cursor":null}""",
                        HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                }
                "/v1/transactions" -> {
                    postKey = request.headers["Idempotency-Key"]
                    respond("{}", HttpStatusCode.InternalServerError, headersOf(HttpHeaders.ContentType, "application/json"))
                }
                else -> error("Unexpected path: ${request.url.encodedPath}")
            }
        }
        val api = ApiClient(engine, "http://localhost")
        val pending = PendingTransaction(OwnerId(0, 7),
            TransactionCreateCommand.Expense(Money(1234, Currency.SEK), BusinessDate(2026, 11, 8), AccountId(0, 9)),
            IdempotencyKey("original-retry-key"))
        try {
            assertIs<RepositoryResult.Failure>(AuthenticatedRecordingBackend(session(), api).create(pending))
            assertEquals(0, accountReads)
            assertEquals("original-retry-key", postKey)
        } finally { api.close(); engine.close() }
    }

    @Test fun duplicateIdsWithinOnePageFailClosed() = runTest {
        val engine = MockEngine { request ->
            val body = if (request.url.encodedPath == "/v1/me") profile else
                """{"accounts":[${account("9")},${account("9")}],"next_cursor":null}"""
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val result = AuthenticatedRecordingBackend(session(), api).accounts(OwnerId(0, 7))
            assertIs<ApiFailure.InvalidResponse>(assertIs<RepositoryResult.Failure>(result).error)
        } finally { api.close(); engine.close() }
    }

    private fun account(id: String) = """{"id":"00000000-0000-0000-0000-00000000000$id","name":"Daily","account_type":"normal","currency":"SEK","status":"active","balance":"0.00"}"""
    private suspend fun session(): AuthSession {
        val auth = AuthSession(object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("access", "refresh"), 900))
            override suspend fun refresh(credentials: SessionCredentials) = login("", "")
            override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
        }, object : SessionStore {
            override suspend fun read(): SessionCredentials? = null
            override suspend fun replace(credentials: SessionCredentials) {}
            override suspend fun clear() {}
        }) { 0L }
        auth.login("person@example.test", "x")
        return auth
    }
}
