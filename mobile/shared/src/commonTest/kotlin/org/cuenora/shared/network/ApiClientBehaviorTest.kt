package org.cuenora.shared.network

import io.ktor.client.engine.mock.MockEngine
import io.ktor.client.engine.mock.respond
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.config.BackendConfig
import kotlin.test.*
import kotlin.time.Duration.Companion.seconds

class ApiClientBehaviorTest {
    @Test
    fun defaultClientUsesSharedBackendConfiguration() = runTest {
        val engine = MockEngine { request ->
            assertEquals(BackendConfig.BASE_URL.trimEnd('/') + "/v1/categories", request.url.toString())
            respond("null")
        }
        val client = ApiClient(engine)
        try { assertIs<ApiResult.Success>(client.request("/v1/categories")) }
        finally { client.close(); engine.close() }
    }

    @Test
    fun requestHeadersAreNotRetained() = runTest {
        var count = 0
        val engine = MockEngine { request ->
            assertEquals(if (count++ == 0) "Bearer synthetic" else null, request.headers[HttpHeaders.Authorization])
            respond("null")
        }
        val client = ApiClient(engine)
        try {
            client.request("/v1/a", headers = mapOf(HttpHeaders.Authorization to "Bearer synthetic"))
            client.request("/v1/b")
            assertEquals(2, count)
        } finally { client.close(); engine.close() }
    }

    @Test
    fun callerCancellationPropagates() = runTest {
        val started = CompletableDeferred<Unit>()
        val engine = MockEngine { started.complete(Unit); awaitCancellation() }
        val client = ApiClient(engine)
        try {
            val job = async { client.request("/v1/a") }
            started.await()
            job.cancel()
            assertFailsWith<CancellationException> { job.await() }
        } finally { client.close(); engine.close() }
    }

    @Test
    fun requestTimeoutIsActuallyInstalled() = runTest(timeout = 50.seconds) {
        // MockEngine uses its own dispatcher: this verifies the installed real 30-second timeout.
        val engine = MockEngine { delay(40_000); respond("null") }
        val client = ApiClient(engine)
        try { assertEquals(ApiResult.Failure(ApiFailure.Timeout), client.request("/v1/a")) }
        finally { client.close(); engine.close() }
    }

    @Test
    fun correlationPrefersHeaderAndRejectsMalformedBody() = runTest {
        val header = "11111111111111111111111111111111"
        val body = "22222222222222222222222222222222"
        for ((id, expected) in listOf("\"$body\"" to ApiError("future", "safe", body), "null" to null)) {
            val engine = MockEngine {
                respond("""{"code":"future","message":"safe","details":null,"request_id":$id}""",
                    HttpStatusCode.BadRequest, headersOf("X-Request-ID", header))
            }
            val client = ApiClient(engine)
            try {
                assertEquals(ApiResult.Failure(ApiFailure.Http(400, expected, header, null)), client.request("/v1/a"))
            } finally { client.close(); engine.close() }
        }
    }

    @Test
    fun encodedQueryCannotOverrideOriginAndExactNumbersStayText() = runTest {
        val wire = """{"amount":"99999999999999999999999.99","number":99999999999999999999999}"""
        val engine = MockEngine { request ->
            assertEquals(Url(BackendConfig.BASE_URL).host, request.url.host)
            assertEquals("https://other.invalid", request.url.parameters["next"])
            respond(wire)
        }
        val client = ApiClient(engine)
        try {
            val value = assertIs<ApiResult.Success>(client.request("/v1/a?next=https%3A%2F%2Fother.invalid"))
            assertEquals(wire, value.body.toString())
        } finally { client.close(); engine.close() }
    }
}
