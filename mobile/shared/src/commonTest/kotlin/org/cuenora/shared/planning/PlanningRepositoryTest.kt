package org.cuenora.shared.planning

import io.ktor.client.engine.mock.*
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import kotlin.test.*

class PlanningRepositoryTest {
    @Test fun omitsServerOwnedInitialCutoffAndPreservesExplicitThroughAndLargeAmount() = runTest {
        val urls = mutableListOf<String>()
        val (api, engine) = client { request ->
            urls += request.url.encodedPath + request.url.encodedQuery.let { if (it.isEmpty()) "" else "?$it" }
            assertEquals("Bearer synthetic-token", request.headers[HttpHeaders.Authorization])
            respond(body, HttpStatusCode.OK)
        }
        try {
            val repository = HttpPlanningRepository(api, RepositoryContext(OwnerId(1, 2), "synthetic-token"))
            val default = assertIs<RepositoryResult.Success<AccountPlanning>>(repository.load())
            assertEquals("999999999999999999999999999999999999.99", default.value.accounts.single().currentBalance)
            repository.load(BusinessDate(2028, 2, 29))
            assertEquals(listOf("/v1/reports/account-planning", "/v1/reports/account-planning?through=2028-02-29"), urls)
        } finally { api.close(); engine.close() }
    }

    @Test fun malformedOrContradictoryWireDataIsRejected() = runTest {
        val (api, engine) = client { respond(body.replace("\"planning_difference\":\"999999999999999999999999999999999199.99\"", "\"planning_difference\":\"1.00\""), HttpStatusCode.OK) }
        try {
            assertIs<RepositoryResult.Failure>(HttpPlanningRepository(api, RepositoryContext(OwnerId(1, 2), "synthetic-token")).load())
        } finally { api.close(); engine.close() }
    }

    @Test fun explicitCutoffMustMatchResponseCutoff() = runTest {
        val (api, engine) = client { respond(body, HttpStatusCode.OK) }
        try {
            assertIs<RepositoryResult.Failure>(HttpPlanningRepository(api, RepositoryContext(OwnerId(1, 2), "synthetic-token"))
                .load(BusinessDate(2028, 2, 29)))
        } finally { api.close(); engine.close() }
    }

    @Test fun duplicateAccountsAndQuotedBooleanFieldsAreRejected() = runTest {
        val duplicate = body.replace("],\"unassigned\"", "},{\"account_id\":\"00000000-0000-0000-0000-000000000001\",\"name\":\"Old\",\"account_type\":\"savings\",\"status\":\"archived\",\"currency\":\"SEK\",\"current_balance\":\"999999999999999999999999999999999999.99\",\"pending_total\":\"200.00\",\"projected_total\":\"600.00\",\"planning_difference\":\"999999999999999999999999999999999199.99\",\"complete\":true}],\"unassigned\"")
        val quotedBoolean = body.replace("\"complete\":true", "\"complete\":\"true\"")
        for (response in listOf(duplicate, quotedBoolean)) {
            val (api, engine) = client { respond(response, HttpStatusCode.OK) }
            try { assertIs<RepositoryResult.Failure>(HttpPlanningRepository(api, RepositoryContext(OwnerId(1, 2), "synthetic-token")).load()) }
            finally { api.close(); engine.close() }
        }
    }

    @Test fun accountLimitIsEnforced() = runTest {
        val account = body.substringAfter("\"accounts\":[").substringBefore("],\"unassigned\"")
        val response = body.replace("\"accounts\":[${account}]", "\"accounts\":[${(1..1001).joinToString(",") { account }}]")
        val (api, engine) = client { respond(response, HttpStatusCode.OK) }
        try { assertIs<RepositoryResult.Failure>(HttpPlanningRepository(api, RepositoryContext(OwnerId(1, 2), "synthetic-token")).load()) }
        finally { api.close(); engine.close() }
    }

    private fun client(handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData): Pair<ApiClient, MockEngine> {
        val engine = MockEngine(handler)
        return ApiClient(engine, "https://example.invalid") to engine
    }
    private val body = """{"as_of":"2026-11-10T12:00:00Z","through":"2026-11-30","complete":true,"accounts":[{"account_id":"00000000-0000-0000-0000-000000000001","name":"Old","account_type":"savings","status":"archived","currency":"SEK","current_balance":"999999999999999999999999999999999999.99","pending_total":"200.00","projected_total":"600.00","planning_difference":"999999999999999999999999999999999199.99","complete":true}],"unassigned":[{"currency":"SEK","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"EUR","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"USD","pending_total":"0.00","projected_total":"0.00","complete":true}]}"""
}
