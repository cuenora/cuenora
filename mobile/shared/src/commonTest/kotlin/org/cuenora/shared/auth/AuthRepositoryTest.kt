package org.cuenora.shared.auth

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.SessionCredentials
import kotlin.test.*

class AuthRepositoryTest {
    private val pair = """{"access_token":"synthetic-access","refresh_token":"synthetic-refresh","token_type":"Bearer","expires_in":900,"refresh_expires_in":2592000}"""

    @Test fun operationsUseExactContractsWithoutBearerHeadersOrRetries() = runTest {
        val seen = mutableListOf<String>()
        val engine = MockEngine { request ->
            assertEquals(HttpMethod.Post, request.method)
            assertNull(request.headers[HttpHeaders.Authorization])
            seen += request.url.encodedPath
            val body = (request.body as TextContent).text
            when (request.url.encodedPath) {
                "/v1/auth/register" -> {
                    assertEquals("""{"email":"person@example.test","password":"a synthetic password"}""", body)
                    respond("""{"status":"accepted"}""", HttpStatusCode.Accepted)
                }
                "/v1/auth/login" -> {
                    assertEquals("""{"email":"person@example.test","password":" x "}""", body)
                    respond(pair, HttpStatusCode.OK)
                }
                "/v1/auth/refresh" -> {
                    assertEquals("""{"refresh_token":"synthetic-refresh"}""", body)
                    respond(pair, HttpStatusCode.OK)
                }
                else -> {
                    assertEquals("/v1/auth/logout", request.url.encodedPath)
                    assertEquals("""{"refresh_token":"synthetic-refresh"}""", body)
                    respond("", HttpStatusCode.NoContent)
                }
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val repository = HttpAuthRepository(api)
            assertEquals(RepositoryResult.Success(Unit), repository.register("person@example.test", "a synthetic password"))
            val login = assertIs<RepositoryResult.Success<TokenGrant>>(repository.login("person@example.test", " x ")).value
            assertEquals("synthetic-access", login.credentials.accessToken)
            assertEquals(900L, login.expiresInSeconds)
            assertFalse(login.toString().contains("synthetic-access"))
            assertIs<RepositoryResult.Success<TokenGrant>>(repository.refresh(login.credentials))
            assertEquals(RepositoryResult.Success(Unit), repository.logout(login.credentials))
            assertEquals(listOf("/v1/auth/register", "/v1/auth/login", "/v1/auth/refresh", "/v1/auth/logout"), seen)
        } finally { api.close(); engine.close() }
    }

    @Test fun malformedSuccessNeverProducesCredentials() = runTest {
        val invalid = listOf(
            pair.replace("\"Bearer\"", "\"Other\""),
            pair.replace("900", "\"900\""),
            pair.replace("900", "0"),
            pair.replace("900", "1.5"),
            pair.replace("900", "9223372036854775808"),
            pair.replace("synthetic-access", ""),
            pair.replace("synthetic-access", "not a bearer"),
            pair.replace("\"refresh_token\":\"synthetic-refresh\",", ""),
            "{}",
        )
        for (body in invalid) {
            val engine = MockEngine { respond(body, HttpStatusCode.OK) }
            val api = ApiClient(engine, "https://example.invalid")
            try {
                val result = assertIs<RepositoryResult.Failure>(HttpAuthRepository(api).login("x@example.test", "x"))
                assertIs<ApiFailure.InvalidResponse>(result.error)
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun statusAndRegistrationAcknowledgementAreNotAssumed() = runTest {
        for ((status, body) in listOf(200 to """{"status":"accepted"}""", 202 to "{}")) {
            val engine = MockEngine { respond(body, HttpStatusCode.fromValue(status)) }
            val api = ApiClient(engine, "https://example.invalid")
            try { assertIs<RepositoryResult.Failure>(HttpAuthRepository(api).register("x@example.test", "password")) }
            finally { api.close(); engine.close() }
        }
    }

    @Test fun failuresRemainTypedAndCancellationPropagates() = runTest {
        var calls = 0
        val engine = MockEngine { calls++; respond("{}", HttpStatusCode.TooManyRequests, headersOf("Retry-After", "60")) }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val result = assertIs<RepositoryResult.Failure>(HttpAuthRepository(api).refresh(SessionCredentials("a", "r")))
            assertEquals(60L, assertIs<ApiFailure.Http>(result.error).retryAfterSeconds)
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
        val cancelled = MockEngine { throw CancellationException("synthetic cancellation") }
        val cancelledApi = ApiClient(cancelled, "https://example.invalid")
        try { assertFailsWith<CancellationException> { HttpAuthRepository(cancelledApi).login("x@example.test", "x") } }
        finally { cancelledApi.close(); cancelled.close() }
    }
}
