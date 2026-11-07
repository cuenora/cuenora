package org.cuenora.shared.network

import io.ktor.client.HttpClient
import io.ktor.client.HttpClientConfig
import io.ktor.client.engine.HttpClientEngine
import io.ktor.client.network.sockets.ConnectTimeoutException
import io.ktor.client.network.sockets.SocketTimeoutException
import io.ktor.client.plugins.*
import io.ktor.client.request.*
import io.ktor.client.statement.bodyAsText
import io.ktor.http.*
import kotlinx.io.IOException
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.*
import org.cuenora.shared.config.BackendConfig

/** Transport values only; repositories map JSON into the independent domain. */
sealed interface ApiResult {
    data class Success(val status: Int, val body: JsonElement?, val requestId: String?) : ApiResult
    data class Failure(val error: ApiFailure) : ApiResult
}

data class ApiError(val code: String, val message: String, val requestId: String?)

sealed interface ApiFailure {
    data class Http(val status: Int, val error: ApiError?, val requestId: String?, val retryAfterSeconds: Long?) : ApiFailure
    data class InvalidResponse(val status: Int, val requestId: String?) : ApiFailure
    data object Network : ApiFailure
    data object Timeout : ApiFailure
}

class ApiClient(
    engine: HttpClientEngine? = null,
    baseUrl: String = BackendConfig.BASE_URL,
) {
    private val origin = Url(baseUrl).also {
        require(it.protocol == URLProtocol.HTTPS ||
            (it.protocol == URLProtocol.HTTP && it.host in setOf("localhost", "127.0.0.1", "::1"))) {
            "Use HTTPS, or HTTP on loopback for local development"
        }
        require(it.encodedPath in setOf("", "/") && it.parameters.isEmpty() && it.fragment.isEmpty() &&
            it.user == null && it.password == null) { "Base URL must be an origin without credentials" }
    }.toString().trimEnd('/')
    private val client = if (engine == null) HttpClient { configureTransport() }
        else HttpClient(engine) { configureTransport() }

    /** Only API-relative paths; no session, authorization or retry policy is implied. */
    suspend fun request(
        path: String,
        method: HttpMethod = HttpMethod.Get,
        body: JsonElement? = null,
        headers: Map<String, String> = emptyMap(),
    ): ApiResult {
        require(path.startsWith("/v1/")) { "Expected a /v1/ API path" }
        return try {
            val response = client.request(origin + path) {
                this.method = method
                accept(ContentType.Application.Json)
                headers.forEach { (name, value) -> header(name, value) }
                if (body != null) {
                    contentType(ContentType.Application.Json)
                    setBody(body.toString())
                }
            }
            val status = response.status.value
            val requestId = response.headers["X-Request-ID"]?.takeIf { requestIdPattern.matches(it) }
            val text = response.bodyAsText()
            if (status !in 200..299) {
                val error = parseError(text)
                val retry = response.headers[HttpHeaders.RetryAfter]
                    ?.takeIf { it.isNotEmpty() && it.all { c -> c in '0'..'9' } }
                    ?.toLongOrNull()?.takeIf { it > 0 }
                ApiResult.Failure(ApiFailure.Http(status, error, requestId ?: error?.requestId, retry))
            } else if (status == 204) {
                ApiResult.Success(status, null, requestId)
            } else {
                try {
                    val json = Json.parseToJsonElement(text)
                    if (json.isWireJson()) ApiResult.Success(status, json, requestId)
                    else ApiResult.Failure(ApiFailure.InvalidResponse(status, requestId))
                } catch (_: SerializationException) {
                    ApiResult.Failure(ApiFailure.InvalidResponse(status, requestId))
                }
            }
        } catch (_: HttpRequestTimeoutException) {
            ApiResult.Failure(ApiFailure.Timeout)
        } catch (_: ConnectTimeoutException) {
            ApiResult.Failure(ApiFailure.Timeout)
        } catch (_: SocketTimeoutException) {
            ApiResult.Failure(ApiFailure.Timeout)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: IOException) {
            ApiResult.Failure(ApiFailure.Network)
        }
    }

    /** Close this client. An injected engine remains owned by its caller. */
    fun close() = client.close()
}

private fun HttpClientConfig<*>.configureTransport() {
    expectSuccess = false
    followRedirects = false
    install(HttpTimeout) { requestTimeoutMillis = 30_000 }
}

private val requestIdPattern = Regex("[0-9a-f]{32}")
private val jsonNumber = Regex("-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?")

// Json's tree parser can retain unquoted non-JSON literals; validate without numeric conversion.
private fun JsonElement.isWireJson(): Boolean = when (this) {
    is JsonObject -> values.all { it.isWireJson() }
    is JsonArray -> all { it.isWireJson() }
    is JsonPrimitive -> isString || this == JsonNull || content in setOf("true", "false") || jsonNumber.matches(content)
}

private fun parseError(text: String): ApiError? {
    val value = try { Json.parseToJsonElement(text) as? JsonObject }
        catch (_: SerializationException) { null } ?: return null
    fun string(key: String) = (value[key] as? JsonPrimitive)?.takeIf { it.isString }?.content
    val code = string("code")?.takeIf { it.isNotEmpty() } ?: return null
    val message = string("message")?.takeIf { it.isNotEmpty() } ?: return null
    if (value["details"] != JsonNull || !value.containsKey("request_id")) return null
    val id = if (value["request_id"] == JsonNull) {
        if (code != "request_id_unavailable") return null
        null
    } else string("request_id")?.takeIf { requestIdPattern.matches(it) } ?: return null
    return ApiError(code, message, id)
}
