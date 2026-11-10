package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlin.test.*

class SavingsRepositoryTest {
    private val id = AccountId(0, 1)
    private val context = RepositoryContext(OwnerId(0, 9), "synthetic-bearer")
    private val command = SavingsTermsCommand(BusinessDate(2099, 1, 1), "monthly", true)
    private val body = """{"id":"00000000-0000-0000-0000-000000000002","effective_on":"2099-01-01","payout_frequency":"monthly","capitalize":true,"rate_basis":"nominal_annual","day_count":"actual_calendar_year","balance_timing":"daily_close","rounding":"period_end_half_even"}"""

    @Test fun termsHistoryAndNaturalIdentityWritePreserveExactPolicies() = runTest {
        var calls = 0
        val engine = MockEngine { request ->
            calls++
            assertEquals("Bearer synthetic-bearer", request.headers[HttpHeaders.Authorization])
            assertNull(request.headers["Idempotency-Key"])
            if (request.method == HttpMethod.Get) {
                assertEquals("2098-01-01", request.url.parameters["cursor"])
                respond("""{"versions":[$body],"next_cursor":null}""", HttpStatusCode.OK)
            } else {
                val sent = Json.parseToJsonElement((request.body as TextContent).text).jsonObject
                assertEquals(JsonPrimitive("nominal_annual"), sent["rate_basis"])
                assertEquals(JsonPrimitive(true), sent["capitalize"])
                respond(body, HttpStatusCode.Created)
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val repo = HttpSavingsRepository(api, context)
            val page = assertIs<RepositoryResult.Success<Page<SavingsTermsVersion>>>(repo.terms(id, SavingsHistoryQuery(50, BusinessDate(2098, 1, 1))))
            assertEquals(command, page.value.items.single().terms)
            assertEquals(command, assertIs<RepositoryResult.Success<SavingsTermsVersion>>(repo.appendTerms(id, command)).value.terms)
            assertEquals(2, calls)
        } finally { api.close(); engine.close() }
    }

    @Test fun ratesRetainPrecisionAndRejectMismatchedAcknowledgements() = runTest {
        val engine = MockEngine { request ->
            assertEquals(JsonPrimitive("3.123456"), Json.parseToJsonElement((request.body as TextContent).text).jsonObject["nominal_annual_percent"])
            respond("""{"id":"00000000-0000-0000-0000-000000000002","effective_on":"2099-01-02","nominal_annual_percent":"3.123456"}""", HttpStatusCode.Created)
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            assertIs<RepositoryResult.Failure>(HttpSavingsRepository(api, context).appendRate(id, SavingsRateCommand(BusinessDate(2099, 1, 1), "3.123456")))
            assertFailsWith<IllegalArgumentException> { SavingsRateCommand(BusinessDate(2099, 1, 1), "3.1234567") }
        } finally { api.close(); engine.close() }
    }

    @Test fun forecastRequiresExplicitRangeAndDoesNotPost() = runTest {
        var calls = 0
        val engine = MockEngine { request ->
            calls++
            assertEquals(HttpMethod.Get, request.method)
            assertEquals("2099-01-01", request.url.parameters["start_on"])
            assertEquals("2099-01-31", request.url.parameters["end_on"])
            respond("{}", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            assertIs<RepositoryResult.Failure>(HttpSavingsRepository(api, context).forecast(id, BusinessDate(2099, 1, 1), BusinessDate(2099, 1, 31)))
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
    }
    @Test fun forecastPreservesArbitraryPrecisionAndOpenTails() = runTest {
        val termsJson = body.replace("\"id\":\"00000000-0000-0000-0000-000000000002\",", "")
        val body = """{"account_id":"00000000-0000-0000-0000-000000000001","currency":"SEK","start_on":"2099-01-01","end_on":"2099-01-02","balance_as_of":"2026-11-10","posted_balance":"1000.00","balance_assumption":"constant_posted_snapshot","includes_prior_accrual":false,"partial_first_period":false,"terms":$termsJson,"rate_segments":[{"start_on":"2099-01-01","end_on":"2099-01-02","effective_on":"2099-01-01","nominal_annual_percent":"3.123456","days_in_year":365}],"periods":[{"start_on":"2099-01-01","end_on":"2099-01-02","exact_accrual_minor_units":{"numerator":"999999999999999999999999999999","denominator":"7"},"projected_payout":null}]}"""
        val engine = MockEngine { respond(body, HttpStatusCode.OK) }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val estimate = assertIs<RepositoryResult.Success<SavingsEstimate>>(HttpSavingsRepository(api, context).forecast(id, BusinessDate(2099, 1, 1), BusinessDate(2099, 1, 2))).value
            assertEquals("999999999999999999999999999999", estimate.periods.single().accrual.numerator)
            assertNull(estimate.periods.single().projectedPayout)
            assertEquals(Money(100000, Currency.SEK), estimate.postedBalance)
            assertFalse(estimate.includesPriorAccrual)
        } finally { api.close(); engine.close() }
    }

}
