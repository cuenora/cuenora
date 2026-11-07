package org.cuenora.shared.network

import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.client.plugins.HttpRequestTimeoutException
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.io.IOException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import kotlin.test.*

class ApiClientTest {
    private val id = "0123456789abcdef0123456789abcdef"
    private fun envelope(code: String = "email_not_verified", requestId: String = "\"$id\"") =
        """{"code":"$code","message":"Safe message.","details":null,"request_id":$requestId}"""

    @Test
    fun defaultEngineAndInjectedEngineHaveExplicitLifecycles() = runTest {
        ApiClient().close() // Resolve the real target engine, without a request.
        val engine = MockEngine { respond("null") }
        val first = ApiClient(engine)
        first.close()
        val second = ApiClient(engine)
        try { assertIs<ApiResult.Success>(second.request("/v1/example")) }
        finally { second.close(); engine.close() }
    }

    @Test
    fun configurationRejectsRemoteCleartextAndNonApiPaths() = runTest {
        for (origin in listOf("http://example.invalid", "https://user:secret@example.invalid",
            "https://example.invalid/prefix", "https://example.invalid?key=value")) {
            assertFailsWith<IllegalArgumentException> { ApiClient(baseUrl = origin) }
        }
        val engine = MockEngine { error("Invalid paths must not issue a request") }
        val client = ApiClient(engine, "http://127.0.0.1:3000")
        try { assertFailsWith<IllegalArgumentException> { client.request("https://other.invalid/v1/example") } }
        finally { client.close(); engine.close() }
    }

    @Test
    fun jsonRequestsPreserveExactStringsAndResponseMetadata() = runTest {
        val json = Json.parseToJsonElement("""{"amount":"92233720368547758.07"}""")
        val engine = MockEngine { request ->
            assertEquals("https://example.invalid/v1/example?limit=2", request.url.toString())
            assertEquals(HttpMethod.Post, request.method)
            assertEquals("Bearer synthetic", request.headers[HttpHeaders.Authorization])
            assertEquals("test-key", request.headers["Idempotency-Key"])
            assertEquals(json.toString(), (request.body as TextContent).text)
            assertEquals(ContentType.Application.Json, request.body.contentType)
            assertEquals("application/json", request.headers[HttpHeaders.Accept])
            respond(json.toString(), headers = headersOf("X-Request-ID", id))
        }
        val client = ApiClient(engine, "https://example.invalid/")
        try {
            assertEquals(ApiResult.Success(200, json, id), client.request("/v1/example?limit=2", HttpMethod.Post, json,
                mapOf(HttpHeaders.Authorization to "Bearer synthetic", "Idempotency-Key" to "test-key")))
        } finally { client.close(); engine.close() }
    }

    @Test
    fun noContentAndMalformedSuccessAreDistinct() = runTest {
        for ((status, body, expected) in listOf(
            Triple(HttpStatusCode.NoContent, "", ApiResult.Success(204, null, id)),
            Triple(HttpStatusCode.OK, "<html>oops</html>", ApiResult.Failure(ApiFailure.InvalidResponse(200, id))),
            Triple(HttpStatusCode.OK, "{\"amount\":NaN}", ApiResult.Failure(ApiFailure.InvalidResponse(200, id))),
        )) {
            val engine = MockEngine { respond(body, status, headersOf("X-Request-ID", id)) }
            val client = ApiClient(engine)
            try { assertEquals(expected, client.request("/v1/example")) }
            finally { client.close(); engine.close() }
        }
    }

    @Test
    fun httpFailuresPreserveKnownAndFutureCodesWithoutRetrying() = runTest {
        for ((status, code) in listOf(401 to "invalid_access_token", 403 to "email_not_verified",
            409 to "idempotency_conflict", 429 to "rate_limited", 503 to "future_code")) {
            var calls = 0
            val engine = MockEngine {
                calls++
                respond(envelope(code), HttpStatusCode.fromValue(status), headersOf(
                    "X-Request-ID" to listOf(id), HttpHeaders.RetryAfter to listOf("30")))
            }
            val client = ApiClient(engine)
            try {
                val failure = assertIs<ApiResult.Failure>(client.request("/v1/example")).error
                assertEquals(ApiFailure.Http(status, ApiError(code, "Safe message.", id), id, 30), failure)
                assertEquals(1, calls)
            } finally { client.close(); engine.close() }
        }
    }

    @Test
    fun malformedErrorsAndRedirectsRemainHttpFailures() = runTest {
        for (body in listOf("<html>upstream</html>", "{}", envelope(requestId = "42"),
            envelope().replace("\"details\":null,", ""), envelope().replace("\"message\":\"Safe message.\"", "\"message\":42"))) {
            val engine = MockEngine { respond(body, HttpStatusCode.BadGateway, headersOf(
                "X-Request-ID" to listOf(id), HttpHeaders.RetryAfter to listOf("-1"))) }
            val client = ApiClient(engine)
            try { assertEquals(ApiResult.Failure(ApiFailure.Http(502, null, id, null)), client.request("/v1/example")) }
            finally { client.close(); engine.close() }
        }
        var calls = 0
        val engine = MockEngine {
            calls++
            respond("", HttpStatusCode.TemporaryRedirect, headersOf(HttpHeaders.Location, "https://other.invalid/v1/example"))
        }
        val client = ApiClient(engine)
        try {
            assertEquals(ApiResult.Failure(ApiFailure.Http(307, null, null, null)), client.request("/v1/example"))
            assertEquals(1, calls)
        } finally { client.close(); engine.close() }
    }

    @Test
    fun entropyFailureAndInvalidRetryHeadersDoNotInventMetadata() = runTest {
        for (retry in listOf("", "-1", "0", "1.5", "999999999999999999999", "Wed, 01 Jan 2025 00:00:00 GMT")) {
            val engine = MockEngine { respond(envelope("request_id_unavailable", "null"), HttpStatusCode.ServiceUnavailable,
                headersOf(HttpHeaders.RetryAfter, retry)) }
            val client = ApiClient(engine)
            try { assertEquals(ApiResult.Failure(ApiFailure.Http(503,
                ApiError("request_id_unavailable", "Safe message.", null), null, null)), client.request("/v1/example")) }
            finally { client.close(); engine.close() }
        }
    }

    @Test
    fun transportTimeoutAndCancellationAreNotHttpFailures() = runTest {
        for ((exception, expected) in listOf(
            IOException("private transport diagnostics") to ApiFailure.Network,
            HttpRequestTimeoutException("https://example.invalid", 1) to ApiFailure.Timeout,
        )) {
            val engine = MockEngine { throw exception }
            val client = ApiClient(engine)
            try { assertEquals(ApiResult.Failure(expected), client.request("/v1/example")) }
            finally { client.close(); engine.close() }
        }
        val engine = MockEngine { throw CancellationException("cancelled") }
        val client = ApiClient(engine)
        try { assertFailsWith<CancellationException> { client.request("/v1/example") } }
        finally { client.close(); engine.close() }
    }
}
