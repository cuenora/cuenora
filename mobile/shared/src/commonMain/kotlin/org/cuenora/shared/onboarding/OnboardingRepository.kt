package org.cuenora.shared.onboarding

import io.ktor.http.*
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*

/** Read-only server identity, not client-granted financial authority. */
data class OnboardingProfile(val ownerId: OwnerId, val emailVerified: Boolean, val timeZone: String?)

interface OnboardingRepository {
    suspend fun profile(): RepositoryResult<OnboardingProfile>
    suspend fun requestVerification(): RepositoryResult<Unit>
    suspend fun confirmVerification(token: String): RepositoryResult<Unit>
    suspend fun confirmZone(zone: String): RepositoryResult<String>
}

/** Construct inside the session's serialized authorized operation; never retain a stale bearer. */
class HttpOnboardingRepository(private val api: ApiClient, private val bearer: String) : OnboardingRepository {
    private fun headers() = mapOf(HttpHeaders.Authorization to "Bearer $bearer")
    override suspend fun profile() = mapResponse(api.request("/v1/me", headers = headers()), 200) {
        val root = it.jsonObject
        val id = uuidBits(root.string("id"))
        val verified = root.getValue("email_verified").jsonPrimitive
        require(!verified.isString && verified.content in setOf("true", "false"))
        val zone = root.nullableString("time_zone")
        require(zone == null || zone.isNotBlank() && zone.length <= 255)
        OnboardingProfile(OwnerId(id.first, id.second), verified.boolean, zone)
    }
    override suspend fun requestVerification() = mapResponse(api.request(
        "/v1/auth/email-verification/request", HttpMethod.Post, buildJsonObject {}, headers()), 202) {
        require(it.jsonObject.string("status") == "accepted")
    }
    override suspend fun confirmVerification(token: String): RepositoryResult<Unit> = when (val result = api.request(
        "/v1/auth/email-verification/confirm", HttpMethod.Post, buildJsonObject { put("token", token) }, headers())) {
        is ApiResult.Failure -> RepositoryResult.Failure(result.error)
        is ApiResult.Success -> if (result.status == 204) RepositoryResult.Success(Unit)
            else RepositoryResult.Failure(ApiFailure.InvalidResponse(result.status, result.requestId))
    }
    override suspend fun confirmZone(zone: String) = mapResponse(api.request(
        "/v1/me", HttpMethod.Patch, buildJsonObject { put("time_zone", zone) }, headers()), 200) {
        it.jsonObject.string("time_zone").also { saved -> require(saved == zone) }
    }
}
