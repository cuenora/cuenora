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

class IndependentOwnerBackendTest {
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""

    @Test fun changedOwnerNeverPostsOrReadsAccountsAndDoesNotClearNewCredentials() = runTest {
        var current = profile
        var posts = 0
        var reads = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/transactions") posts++
            if (request.url.encodedPath == "/v1/accounts") reads++
            respond(current, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        val auth = session()
        try {
            val backend = AuthenticatedRecordingBackend(auth, api)
            assertIs<RepositoryResult.Success<*>>(backend.profile())
            current = profile.replace("000000000007", "000000000008")
            assertIs<RepositoryResult.Failure>(backend.accounts(OwnerId(0,7)))
            val pending = PendingTransaction(OwnerId(0,7), TransactionCreateCommand.Expense(Money(100,Currency.SEK), BusinessDate(2026,11,8)),IdempotencyKey("original"))
            assertIs<RepositoryResult.Failure>(backend.create(pending))
            assertEquals(0,posts);assertEquals(0,reads)
            assertEquals(AuthPhase.SignedIn,auth.state.value.phase)
        } finally {api.close();engine.close()}
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
