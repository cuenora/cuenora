package org.cuenora.shared.insights

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class AuthenticatedInsightsReaderTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }
    private fun session(store: Store = Store()) = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(
            TokenGrant(SessionCredentials("insights-access", "insights-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, store) { 0L }

    private val profile = """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""
    private val flows = """{"date_from":"2024-02-01","date_to":"2024-02-29","currencies":[{"currency":"SEK","income":"92233720368547758.08","expenses":"0.00","net_flow":"92233720368547758.08"},{"currency":"EUR","income":"0.00","expenses":"0.00","net_flow":"0.00"},{"currency":"USD","income":"0.00","expenses":"0.00","net_flow":"0.00"}]}"""
    private val monthly = """{"date_from":"2024-02-01","date_to":"2024-02-29","currencies":[{"currency":"SEK","months":[{"month":"2024-02","date_from":"2024-02-01","date_to":"2024-02-29","income":"92233720368547758.08","expenses":"0.00","net_flow":"92233720368547758.08"}]},{"currency":"EUR","months":[{"month":"2024-02","date_from":"2024-02-01","date_to":"2024-02-29","income":"0.00","expenses":"0.00","net_flow":"0.00"}]},{"currency":"USD","months":[{"month":"2024-02","date_from":"2024-02-01","date_to":"2024-02-29","income":"0.00","expenses":"0.00","net_flow":"0.00"}]}]}"""
    private fun distribution(total: String = "0.00", percentage: String? = null): String {
        val categories = (1..7).joinToString(",") { n ->
            """{"category_id":"00000000-0000-0000-0000-${n.toString().padStart(12, '0')}","code":"category_$n","name":"Category $n","expenses":"0.00","percentage":${percentage?.let { "\"$it\"" } ?: "null"}}"""
        } + "," + """{"category_id":null,"code":null,"name":null,"expenses":"$total","percentage":${percentage?.let { "\"$it\"" } ?: "null"}}"""
        return """{"date_from":"2024-02-01","date_to":"2024-02-29","currencies":[{"currency":"SEK","expenses":"$total","categories":[$categories]},{"currency":"EUR","expenses":"0.00","categories":[${categories.replace("\"$total\"", "\"0.00\"").replace("\"percentage\":\"100.00\"", "\"percentage\":null")}]},{"currency":"USD","expenses":"0.00","categories":[${categories.replace("\"$total\"", "\"0.00\"").replace("\"percentage\":\"100.00\"", "\"percentage\":null")}]}]}"""
    }

    @Test fun readsExactIndependentReportsAndPreservesArbitraryPrecisionStrings() = runTest {
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath
            assertEquals("Bearer insights-access", request.headers[HttpHeaders.Authorization])
            respond(when (request.url.encodedPath) {
                "/v1/me" -> profile
                "/v1/reports/flows" -> flows
                "/v1/reports/monthly-flows" -> monthly
                else -> distribution()
            }, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val result = assertIs<RepositoryResult.Success<InsightsReport>>(
                AuthenticatedInsightsReader(auth, api) { error("explicit range does not need the profile clock") }
                    .load(InsightsDateRange("2024-02-01", "2024-02-29")))
            assertEquals(listOf("/v1/me", "/v1/reports/flows", "/v1/reports/monthly-flows", "/v1/reports/expense-distribution"), paths)
            assertEquals(listOf(Currency.SEK, Currency.EUR, Currency.USD), result.value.currencies.map { it.currency })
            assertEquals("92233720368547758.08", result.value.currencies.first().income)
            assertEquals("2024-02-29", result.value.currencies.first().months.single().dateTo)
            assertNull(result.value.currencies.first().categories.last().percentage)
        } finally { api.close(); engine.close() }
    }

    @Test fun defaultRangeUsesOneConfirmedProfileZoneDateAndUnavailableZoneFailsClosed() = runTest {
        var calls = 0
        val engine = MockEngine { request ->
            calls++
            respond(profile.replace("Europe/Stockholm", "Europe/Stockholm"), HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            var zone: String? = null
            val result = AuthenticatedInsightsReader(auth, api) { zone.also { zone = it } }.load(null)
            assertIs<RepositoryResult.Failure>(result)
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
    }

    @Test fun defaultRangeUsesOneProfileZoneDateForAllThreeReports() = runTest {
        val requestedDates = mutableListOf<String?>()
        val engine = MockEngine { request ->
            requestedDates += request.url.parameters["date_from"]
            when (request.url.encodedPath) {
                "/v1/me" -> respond(profile, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/reports/flows" -> respond(flows, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/reports/monthly-flows" -> respond(monthly, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                else -> respond(distribution(), HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val zones = mutableListOf<String>()
            val result = AuthenticatedInsightsReader(auth, api) { zone -> zones += zone; "2024-02-12" }.load(null)
            val report = assertIs<RepositoryResult.Success<InsightsReport>>(result).value
            assertEquals(listOf("Europe/Stockholm"), zones)
            assertEquals(InsightsDateRange("2024-02-01", "2024-02-29"), report.range)
            assertEquals(listOf(null, "2024-02-01", "2024-02-01", "2024-02-01"), requestedDates)
        } finally { api.close(); engine.close() }
    }

    @Test fun december9999MonthlyRangeDoesNotConstructAnOutOfRangeFollowingYear() = runTest {
        val engine = MockEngine { request ->
            val body = when (request.url.encodedPath) {
                "/v1/me" -> profile
                "/v1/reports/flows" -> flows.replace("2024-02-01", "9999-12-01").replace("2024-02-29", "9999-12-31")
                "/v1/reports/monthly-flows" -> monthly.replace("2024-02-01", "9999-12-01")
                    .replace("2024-02-29", "9999-12-31").replace("2024-02", "9999-12")
                else -> distribution().replace("2024-02-01", "9999-12-01").replace("2024-02-29", "9999-12-31")
            }
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val report = assertIs<RepositoryResult.Success<InsightsReport>>(
                AuthenticatedInsightsReader(auth, api) { "9999-12-15" }.load(null)).value
            assertEquals("9999-12-31", report.currencies.first().months.single().dateTo)
        } finally { api.close(); engine.close() }
    }

    @Test fun unverifiedProfileAndOwnerSwitchDoNotReadAnotherOwnersReports() = runTest {
        var switched = false
        var unverified = false
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath
            when (request.url.encodedPath) {
                "/v1/me" -> respond(
                    if (switched) profile.replace("000007", "000008") else if (unverified) profile.replace("true", "false") else profile,
                    HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/reports/flows" -> respond(flows, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/reports/monthly-flows" -> respond(monthly, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                else -> respond(distribution(), HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedInsightsReader(auth, api) { null }
            assertIs<RepositoryResult.Success<InsightsReport>>(reader.load(InsightsDateRange("2024-02-01", "2024-02-29")))
            val reportRequests = paths.count { it.startsWith("/v1/reports/") }
            switched = true
            assertEquals(403, (assertIs<RepositoryResult.Failure>(reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))).error as ApiFailure.Http).status)
            assertEquals(reportRequests, paths.count { it.startsWith("/v1/reports/") })
            switched = false; unverified = true
            assertEquals(403, (assertIs<RepositoryResult.Failure>(reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))).error as ApiFailure.Http).status)
            assertEquals(reportRequests, paths.count { it.startsWith("/v1/reports/") })
        } finally { api.close(); engine.close() }
    }

    @Test fun clearDuringSuspendedProfileDoesNotContinueAndDoesNotExpireValidSession() = runTest {
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath
            if (request.url.encodedPath == "/v1/me") {
                entered.complete(Unit); release.await()
                respond(profile, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            } else error("resource read must not start after clear")
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedInsightsReader(auth, api) { null }
            val result = CompletableDeferred<RepositoryResult<InsightsReport>>()
            val request = launch { result.complete(reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))) }
            entered.await(); reader.clear(); release.complete(Unit); request.join()
            assertEquals(403, (assertIs<RepositoryResult.Failure>(result.await()).error as ApiFailure.Http).status)
            assertEquals(listOf("/v1/me"), paths)
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }

    @Test fun clearDuringSuspendedReportStopsFollowingReads() = runTest {
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath
            when (request.url.encodedPath) {
                "/v1/me" -> respond(profile, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                "/v1/reports/flows" -> {
                    entered.complete(Unit); release.await()
                    respond(flows, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
                }
                else -> error("later independent report must not start after clear")
            }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedInsightsReader(auth, api) { null }
            val result = CompletableDeferred<RepositoryResult<InsightsReport>>()
            val request = launch { result.complete(reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))) }
            entered.await(); reader.clear(); release.complete(Unit); request.join()
            assertEquals(403, (assertIs<RepositoryResult.Failure>(result.await()).error as ApiFailure.Http).status)
            assertEquals(listOf("/v1/me", "/v1/reports/flows"), paths)
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }

    @Test fun categoryTotalRemainsSeparateWhenReportsObserveDifferentData() = runTest {
        val engine = MockEngine { request ->
            val body = when (request.url.encodedPath) {
                "/v1/me" -> profile
                "/v1/reports/flows" -> flows
                "/v1/reports/monthly-flows" -> monthly
                else -> distribution("0.01", "100.00")
            }
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val report = assertIs<RepositoryResult.Success<InsightsReport>>(
                AuthenticatedInsightsReader(auth, api) { null }.load(InsightsDateRange("2024-02-01", "2024-02-29"))).value
            assertEquals("0.00", report.currencies.first().expenses)
            assertEquals("0.01", report.currencies.first().distributionExpenses)
        } finally { api.close(); engine.close() }
    }

    @Test fun failedReportNeverBecomesZeroAndKeeps401SessionExpiration() = runTest {
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/me") respond(profile, HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
            else respond("{}", HttpStatusCode.Unauthorized, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val store = Store(); val auth = session(store); auth.login("person@example.test", "x")
            val result = AuthenticatedInsightsReader(auth, api) { null }.load(InsightsDateRange("2024-02-01", "2024-02-29"))
            assertEquals(401, (assertIs<RepositoryResult.Failure>(result).error as ApiFailure.Http).status)
            assertEquals(AuthPhase.SignedOut, auth.state.value.phase)
            assertNull(store.value)
        } finally { api.close(); engine.close() }
    }

    @Test fun failedFirstReportShortCircuitsRemainingEndpointsAndReaderClearIsPermanent() = runTest {
        val paths = mutableListOf<String>()
        val engine = MockEngine { request ->
            paths += request.url.encodedPath
            if (request.url.encodedPath == "/v1/me") respond(profile, HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
            else respond("{}", HttpStatusCode.ServiceUnavailable, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val reader = AuthenticatedInsightsReader(auth, api) { null }
            val result = reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))
            assertEquals(503, (assertIs<RepositoryResult.Failure>(result).error as ApiFailure.Http).status)
            assertEquals(listOf("/v1/me", "/v1/reports/flows"), paths)
            reader.clear()
            assertEquals(403, (assertIs<RepositoryResult.Failure>(
                reader.load(InsightsDateRange("2024-02-01", "2024-02-29"))).error as ApiFailure.Http).status)
            assertEquals(listOf("/v1/me", "/v1/reports/flows"), paths)
            assertEquals(AuthPhase.SignedIn, auth.state.value.phase)
        } finally { api.close(); engine.close() }
    }
}
