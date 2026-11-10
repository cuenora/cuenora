package org.cuenora.shared.independent

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.insights.*
import org.cuenora.shared.planning.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class DashboardBoundaryProbe {
    private class Store : SessionStore {
        var credentials: SessionCredentials? = null
        override suspend fun read() = credentials
        override suspend fun replace(credentials: SessionCredentials) { this.credentials = credentials }
        override suspend fun clear() { credentials = null }
    }
    private fun auth(store: Store) = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("probe-access", "probe-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, store) { 0L }
    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""
    private val headers = headersOf(HttpHeaders.ContentType, "application/json")
    private val range = InsightsDateRange("2024-02-01", "2024-02-29")

    @Test fun retiredPlanningProfile401CannotClearGoodCredentials() = runTest { retiredFailure(false) }
    @Test fun retiredInsightsProfile401CannotClearGoodCredentials() = runTest { retiredFailure(true) }
    private suspend fun retiredFailure(insights: Boolean) = coroutineScope {
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val engine = MockEngine { request ->
            assertEquals(HttpMethod.Get, request.method)
            assertEquals("/v1/me", request.url.encodedPath)
            entered.complete(Unit); release.await()
            respond("{}", HttpStatusCode.Unauthorized, headers)
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val store = Store(); val auth = auth(store); auth.login("probe@example.test", "x")
            val before = store.credentials.hashCode()
            val planning = AuthenticatedPlanningReader(auth, api)
            val reader = AuthenticatedInsightsReader(auth, api) { "2024-02-15" }
            val pending = async { if (insights) reader.load(range) else planning.load() }
            entered.await()
            if (insights) reader.clear() else planning.clear()
            release.complete(Unit); pending.await()
            assertEquals(before, store.credentials.hashCode(), "retired response must not mutate active credential store")
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }

    @Test fun freshOwnerIsPinnedEvenWhenFirstReportUnavailable() = runTest {
        for (insights in listOf(false, true)) {
            var switched = false; var reportCalls = 0
            val engine = MockEngine { request ->
                assertEquals(HttpMethod.Get, request.method)
                if (request.url.encodedPath == "/v1/me") respond(if (switched) profile.replace("000007", "000008") else profile, HttpStatusCode.OK, headers)
                else { reportCalls++; respond("{}", HttpStatusCode.ServiceUnavailable, headers) }
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = auth(store); auth.login("probe@example.test", "x")
                val before = store.credentials.hashCode()
                val planning = AuthenticatedPlanningReader(auth, api)
                val reader = AuthenticatedInsightsReader(auth, api) { "2024-02-15" }
                suspend fun load() = if (insights) reader.load(range) else planning.load()
                assertEquals(503, (assertIs<RepositoryResult.Failure>(load()).error as ApiFailure.Http).status)
                switched = true
                assertEquals(403, (assertIs<RepositoryResult.Failure>(load()).error as ApiFailure.Http).status)
                assertEquals(1, reportCalls)
                assertEquals(before, store.credentials.hashCode())
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun logoutDuringSuspendedReportClearsAndPreventsReuse() = runTest {
        for (insights in listOf(false, true)) {
            val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
            var reportCalls = 0
            val engine = MockEngine { request ->
                assertEquals(HttpMethod.Get, request.method)
                if (request.url.encodedPath == "/v1/me") respond(profile, HttpStatusCode.OK, headers)
                else { reportCalls++; entered.complete(Unit); release.await(); respond("{}", HttpStatusCode.ServiceUnavailable, headers) }
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val store = Store(); val auth = auth(store); auth.login("probe@example.test", "x")
                val planning = AuthenticatedPlanningReader(auth, api)
                val reader = AuthenticatedInsightsReader(auth, api) { "2024-02-15" }
                suspend fun load() = if (insights) reader.load(range) else planning.load()
                val pending = async { load() }
                entered.await()
                val logout = launch { auth.logout() }
                if (insights) reader.clear() else planning.clear()
                release.complete(Unit); pending.await(); logout.join()
                assertNull(store.credentials)
                assertEquals(AuthPhase.SignedOut, auth.state.value.phase)
                assertEquals(401, (assertIs<RepositoryResult.Failure>(load()).error as ApiFailure.Http).status)
                assertEquals(1, reportCalls)
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun rejectedDateDraftDoesNotChangeSelectedReportOrCallReader() = runTest {
        var reads = 0
        val session = InsightsSession(InsightsReader { reads++; RepositoryResult.Success(InsightsReport(range, emptyList())) })
        session.loadIfNeeded()
        val before = session.state.value.report.hashCode()
        session.editDates(); session.changeDateDraft("2024-02-30", "2024-02-29")
        assertFalse(session.applyDateDraft())
        assertEquals(before, session.state.value.report.hashCode())
        assertEquals(range, session.state.value.range)
        assertEquals(1, reads)
        session.cancelDateDraft()
        assertEquals("2024-02-01", session.state.value.draftFrom)
        session.loadIfNeeded(); assertEquals(1, reads)
    }
}
