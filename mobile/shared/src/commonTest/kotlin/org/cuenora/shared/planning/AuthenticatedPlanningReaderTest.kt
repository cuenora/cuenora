package org.cuenora.shared.planning

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthenticatedPlanningReaderTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }
    private fun session(store: Store = Store()) = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-access", "synthetic-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, store) { 0L }
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":null}"""
    private val body = """{"as_of":"2026-11-10T12:00:00Z","through":"2026-11-30","complete":true,"accounts":[],"unassigned":[{"currency":"SEK","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"EUR","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"USD","pending_total":"0.00","projected_total":"0.00","complete":true}]}"""

    @Test fun bindsVerifiedOwnerAndRejectsOwnerSwitchOrUnverifiedProfile() = runTest {
        var changed = false
        var verified = true
        var reportRequests = 0
        val engine = MockEngine { request ->
            val isProfile = request.url.encodedPath == "/v1/me"
            if (!isProfile) reportRequests++
            val current = when {
                changed -> profile.replace("000007", "000008")
                !verified -> profile.replace("true", "false")
                else -> profile
            }
            respond(if (isProfile) current else body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedPlanningReader(auth, api)
            assertIs<RepositoryResult.Success<AccountPlanning>>(reader.load())
            changed = true
            assertEquals(403, (assertIs<RepositoryResult.Failure>(reader.load()).error as ApiFailure.Http).status)
            verified = false; changed = false
            assertEquals(403, (assertIs<RepositoryResult.Failure>(reader.load()).error as ApiFailure.Http).status)
            assertEquals(1, reportRequests)
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }

    @Test fun profileAndResource401ExpireBut403DoesNot() = runTest {
        for (resourceStatus in listOf(401, 403)) {
            val engine = MockEngine { request ->
                if (request.url.encodedPath == "/v1/me") respond(profile, HttpStatusCode.OK)
                else respond("{}", HttpStatusCode.fromValue(resourceStatus))
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
                val result = assertIs<RepositoryResult.Failure>(AuthenticatedPlanningReader(auth, api).load())
                assertEquals(resourceStatus, (result.error as ApiFailure.Http).status)
                assertEquals(if (resourceStatus == 401) AuthPhase.SignedOut else AuthPhase.SignedIn, auth.state.value.phase)
                assertEquals(resourceStatus == 401, store.value == null)
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun retirementDuringProfileOrReportSuspensionPreventsLateData() = runTest {
        for (suspendedPath in listOf("/v1/me", "/v1/reports/account-planning")) {
            val started = CompletableDeferred<Unit>()
            val release = CompletableDeferred<Unit>()
            val engine = MockEngine { request ->
                if (request.url.encodedPath == suspendedPath) { started.complete(Unit); release.await() }
                respond(if (request.url.encodedPath == "/v1/me") profile else body,
                    HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
                val reader = AuthenticatedPlanningReader(auth, api)
                val pending = async { reader.load() }
                started.await(); reader.clear(); release.complete(Unit)
                assertEquals(403, (assertIs<RepositoryResult.Failure>(pending.await()).error as ApiFailure.Http).status)
                assertNotNull(store.value)
                assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
            } finally { api.close(); engine.close() }
        }
    }
}
