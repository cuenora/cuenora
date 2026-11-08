package org.cuenora.shared.onboarding

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.domain.OwnerId
import kotlin.test.*

class OnboardingRepositoryTest {
    @Test fun profileUsesAuthenticatedServerIdentityAndNeverInventsZoneOrVerification() = runTest {
        var calls = 0
        val engine = MockEngine { request ->
            calls++
            assertEquals("/v1/me", request.url.encodedPath)
            assertEquals("Bearer synthetic-token", request.headers[HttpHeaders.Authorization])
            respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":false,"time_zone":null}""",
                HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val profile = assertIs<RepositoryResult.Success<OnboardingProfile>>(
                HttpOnboardingRepository(api, "synthetic-token").profile()).value
            assertEquals(OwnerId(0, 7), profile.ownerId)
            assertFalse(profile.emailVerified)
            assertNull(profile.timeZone)
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
    }

    @Test fun requestConfirmationAndZoneUseExactContractsWithoutAutomaticRetry() = runTest {
        val requests = mutableListOf<Pair<String, String>>()
        val engine = MockEngine { request ->
            assertEquals("Bearer synthetic-token", request.headers[HttpHeaders.Authorization])
            requests += request.url.encodedPath to (request.body as TextContent).text
            when (requests.size) {
                1 -> respond("""{"status":"accepted"}""", HttpStatusCode.Accepted)
                2 -> respond("", HttpStatusCode.NoContent)
                else -> respond("""{"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val repo = HttpOnboardingRepository(api, "synthetic-token")
            assertIs<RepositoryResult.Success<Unit>>(repo.requestVerification())
            assertIs<RepositoryResult.Success<Unit>>(repo.confirmVerification("ev1_synthetic"))
            assertIs<RepositoryResult.Success<String>>(repo.confirmZone("Europe/Stockholm"))
            assertEquals(listOf(
                "/v1/auth/email-verification/request" to "{}",
                "/v1/auth/email-verification/confirm" to "{\"token\":\"ev1_synthetic\"}",
                "/v1/me" to "{\"time_zone\":\"Europe/Stockholm\"}"), requests)
        } finally { api.close(); engine.close() }
    }

    @Test fun malformedProfileAndMismatchedZoneFailClosed() = runTest {
        for (body in listOf(
            """{"id":"bad","email_verified":true,"time_zone":null}""",
            """{"id":"00000000-0000-0000-0000-000000000001","email_verified":"true","time_zone":null}""",
            """{"id":"00000000-0000-0000-0000-000000000001","email_verified":true}""")) {
            val engine = MockEngine { respond(body, HttpStatusCode.OK) }
            val api = ApiClient(engine, "https://example.invalid")
            try { assertIs<RepositoryResult.Failure>(HttpOnboardingRepository(api, "synthetic").profile()) }
            finally { api.close(); engine.close() }
        }
    }
}
