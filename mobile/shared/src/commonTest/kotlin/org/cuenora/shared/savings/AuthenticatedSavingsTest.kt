package org.cuenora.shared.savings

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthenticatedSavingsTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }
    private fun auth() = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-access", "synthetic-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, Store()) { 0L }
    private val owner = OwnerId(0, 7)
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""
    private fun pending() = PendingSavings(owner, "https://example.invalid", SavingsAction.Receipt(AccountId(0, 2), "4.00", Currency.SEK, "2026-11-10"), IdempotencyKey("original_receipt"))

    @Test fun originalReceiptBodyAndKeyAreUnchangedAcrossExplicitRetry() = runTest {
        val posts = mutableListOf<Pair<String?, String>>()
        val engine = MockEngine { request ->
            assertEquals("Bearer synthetic-access", request.headers[HttpHeaders.Authorization])
            if (request.url.encodedPath == "/v1/me") respond(profile, HttpStatusCode.OK)
            else {
                assertEquals("/v1/transactions", request.url.encodedPath)
                posts += request.headers["Idempotency-Key"] to (request.body as TextContent).text
                if (posts.size == 1) respond("{}", HttpStatusCode.ServiceUnavailable)
                else respond("""{"id":"00000000-0000-0000-0000-000000000003","kind":"interest_income","state":"posted","amount":"4.00","currency":"SEK","occurred_on":"2026-11-10","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null}""", HttpStatusCode.Created)
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val session = auth(); session.login("person@example.test", "secret")
            val backend = AuthenticatedSavingsBackend(session, api) { true }
            backend.profile()
            val original = PendingSavings.decode(pending().encode())
            assertIs<RepositoryResult.Failure>(backend.write(original))
            assertIs<RepositoryResult.Success<Unit>>(backend.write(original))
            assertEquals(2, posts.size); assertEquals(posts.first(), posts.last())
            assertEquals("original_receipt", posts.first().first)
            assertEquals("interest_income", Json.parseToJsonElement(posts.first().second).jsonObject["kind"]!!.jsonPrimitive.content)
        } finally { api.close(); engine.close() }
    }
    @Test fun ownerSwitchDoesNotStartWrite() = runTest {
        var foreign = false; var posts = 0
        val engine = MockEngine { request ->
            if (request.method == HttpMethod.Post) { posts++; error("must not post") }
            respond(if (foreign) profile.replace("000000000007", "000000000008") else profile, HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val session = auth(); session.login("person@example.test", "secret")
            val backend = AuthenticatedSavingsBackend(session, api) { true }; backend.profile(); foreign = true
            assertIs<RepositoryResult.Failure>(backend.write(pending()))
            assertEquals(0, posts)
        } finally { api.close(); engine.close() }
    }
    @Test fun retirementDuringFreshProfileReadDoesNotStartWrite() = runTest {
        var active = true; var gate: CompletableDeferred<Unit>? = null
        val started = CompletableDeferred<Unit>(); var posts = 0
        val engine = MockEngine { request ->
            if (request.method == HttpMethod.Post) { posts++; error("must not post") }
            gate?.let { started.complete(Unit); it.await() }
            respond(profile, HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val session = auth(); session.login("person@example.test", "secret")
            val backend = AuthenticatedSavingsBackend(session, api) { active }; backend.profile()
            gate = CompletableDeferred()
            val writing = async { backend.write(pending()) }; started.await(); active = false; gate.complete(Unit)
            assertIs<RepositoryResult.Failure>(writing.await()); assertEquals(0, posts)
        } finally { api.close(); engine.close() }
    }
}
