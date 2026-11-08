package org.cuenora.shared.auth

import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.SessionCredentials

/** Secrets deliberately have no data-class display or generated copying. */
class TokenGrant(val credentials: SessionCredentials, val expiresInSeconds: Long) {
    init { require(expiresInSeconds in 1..Int.MAX_VALUE.toLong()) { "Invalid session lifetime" } }
    override fun toString(): String = "TokenGrant([REDACTED])"
}

interface AuthRepository {
    suspend fun register(email: String, password: String): RepositoryResult<Unit>
    suspend fun login(email: String, password: String): RepositoryResult<TokenGrant>
    suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant>
    suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit>
}

/** Caller owns the client. No retries, persistence, owner identity or financial authority. */
class HttpAuthRepository(private val api: ApiClient) : AuthRepository {
    override suspend fun register(email: String, password: String): RepositoryResult<Unit> =
        mapResponse(api.request("/v1/auth/register", HttpMethod.Post, form(email, password)), 202) {
            require(it.jsonObject.string("status") == "accepted") { "Invalid registration response" }
        }

    override suspend fun login(email: String, password: String): RepositoryResult<TokenGrant> =
        grant(api.request("/v1/auth/login", HttpMethod.Post, form(email, password)))

    override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> =
        grant(api.request("/v1/auth/refresh", HttpMethod.Post, refreshBody(credentials)))

    override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> =
        when (val result = api.request("/v1/auth/logout", HttpMethod.Post, refreshBody(credentials))) {
            is ApiResult.Failure -> RepositoryResult.Failure(result.error)
            is ApiResult.Success -> if (result.status == 204) RepositoryResult.Success(Unit)
                else RepositoryResult.Failure(ApiFailure.InvalidResponse(result.status, result.requestId))
        }

    private fun grant(result: ApiResult): RepositoryResult<TokenGrant> = mapResponse(result, 200) {
        val root = it.jsonObject
        require(root.string("token_type") == "Bearer") { "Invalid token type" }
        fun lifetime(name: String): Long {
            val value = root.getValue(name).jsonPrimitive
            require(!value.isString && Regex("[1-9][0-9]*").matches(value.content)) { "Invalid token lifetime" }
            return value.content.toLong().also { require(it in 1..Int.MAX_VALUE.toLong()) }
        }
        val access = root.string("access_token")
        val refresh = root.string("refresh_token")
        require(listOf(access, refresh).all { token ->
            token.length in 1..4096 && token.all { c -> c.isLetterOrDigit() && c.code < 128 || c in "-._~+/=" }
        }) { "Invalid credential encoding" }
        lifetime("refresh_expires_in")
        TokenGrant(SessionCredentials(access, refresh), lifetime("expires_in"))
    }

    private fun form(email: String, password: String) = buildJsonObject {
        put("email", email)
        put("password", password)
    }

    private fun refreshBody(credentials: SessionCredentials) = buildJsonObject {
        put("refresh_token", credentials.refreshToken)
    }
}
