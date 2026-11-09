package org.cuenora.shared.recurrence

import io.ktor.client.engine.mock.*
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import kotlin.test.*

class RecurrenceRepositoryTest {
    private val context = RepositoryContext(OwnerId(1, 2), "synthetic-token")
    private val rule = """{"id":"00000000-0000-0000-0000-000000000010","amount":"12.34","currency":"SEK","cadence":"monthly","starts_on":"2026-01-31","ends_on":null,"time_zone":"Europe/Stockholm","account_id":null,"category_id":null,"description":null,"state":"active","paused_on":null,"generated_through":null}"""
    private val occurrence = """{"id":"00000000-0000-0000-0000-000000000021","rule_id":"00000000-0000-0000-0000-000000000010","scheduled_on":"2026-01-31","due_on":"2026-02-05","state":"planned","amount":"12.34","currency":"SEK","account_id":null,"category_id":null,"description":null,"transaction_id":null}"""

    private fun api(handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData): Pair<ApiClient, MockEngine> {
        val engine = MockEngine(handler)
        return ApiClient(engine, "https://example.invalid") to engine
    }

    @Test fun listingTreatsCursorAsOpaqueAndDecodesNullableOccurrenceLinkage() = runTest {
        var calls = 0
        val (client, engine) = api { request ->
            calls++
            assertEquals("Bearer synthetic-token", request.headers[HttpHeaders.Authorization])
            if (request.url.encodedPath == "/v1/recurring-rules") {
                assertEquals("limit=50&cursor=opaque%2F%3F%26&state=paused", request.url.encodedQuery)
                respond("""{"rules":[$rule],"next_cursor":"next"}""", HttpStatusCode.OK)
            } else {
                assertEquals("limit=50&state=planned", request.url.encodedQuery)
                respond("""{"occurrences":[$occurrence],"next_cursor":null}""", HttpStatusCode.OK)
            }
        }
        try {
            val repository = HttpRecurrenceRepository(client, context)
            val rules = assertIs<RepositoryResult.Success<Page<RecurrenceRule>>>(repository.rules(cursor = "opaque/?&", state = RuleState.PAUSED))
            assertEquals("next", rules.value.nextCursor)
            assertEquals(RuleId(0, 16), rules.value.items.single().id)
            val occurrences = assertIs<RepositoryResult.Success<Page<RecurrenceOccurrence>>>(repository.occurrences(state = OccurrenceState.PLANNED))
            assertNull(occurrences.value.nextCursor)
            assertNull(occurrences.value.items.single().transactionId)
            assertEquals(BusinessDate(2026, 2, 5), occurrences.value.items.single().dueOn)
            assertEquals(2, calls)
            assertFailsWith<IllegalArgumentException> { repository.rules(limit = 101) }
        } finally { client.close(); engine.close() }
    }

    @Test fun unchangedArchivedRuleAccountIsOmittedFromPatch() = runTest {
        val archived = AccountId(0, 9)
        val current = RecurrenceRule(RuleId(0, 16), Money(1234, Currency.SEK), "monthly", BusinessDate(2026, 1, 31), null,
            "Europe/Stockholm", archived, null, null, RuleState.ACTIVE, null, null)
        val (client, engine) = api { request ->
            assertEquals("PATCH", request.method.value)
            val body = (request.body as TextContent).text
            assertEquals("{\"description\":\"Updated\"}", body)
            respond(rule.replace("\"description\":null", "\"description\":\"Updated\"").replace("\"account_id\":null", "\"account_id\":\"00000000-0000-0000-0000-000000000009\""), HttpStatusCode.OK)
        }
        try {
            val draft = RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-01-31", null,
                "Europe/Stockholm", archived, null, "Updated")
            val result = assertIs<RepositoryResult.Success<RecurrenceRule>>(
                HttpRecurrenceRepository(client, context).edit(current.id, draft, current))
            assertEquals(archived, result.value.accountId)
        } finally { client.close(); engine.close() }
    }

    @Test fun confirmedActualPaymentSupportsAccountlessAndAssignedExpenses() = runTest {
        val account = AccountId(0, 9)
        val bodies = mutableListOf<String>()
        val (client, engine) = api { request ->
            assertEquals("/v1/recurring-occurrences/00000000-0000-0000-0000-000000000021/confirm", request.url.encodedPath)
            assertEquals(HttpMethod.Post, request.method)
            assertEquals("confirmation_key", request.headers["Idempotency-Key"])
            bodies += (request.body as TextContent).text
            val encodedAccount = if (bodies.size == 1) "null" else "\"00000000-0000-0000-0000-000000000009\""
            respond("""{"id":"00000000-0000-0000-0000-000000000042","kind":"expense","state":"posted","amount":"5.25","currency":"EUR","occurred_on":"2026-11-01","account_id":$encodedAccount,"category_id":null,"description":"Actual"}""", HttpStatusCode.Created)
        }
        try {
            val repository = HttpRecurrenceRepository(client, context)
            val accountless = ActualPaymentDraft("5.25", Currency.EUR, "2026-11-01", description = "Actual")
            val assigned = accountless.copy(accountId = account)
            assertIs<RepositoryResult.Success<TransactionRecord>>(repository.confirm(OccurrenceId(0, 33), accountless, IdempotencyKey("confirmation_key")))
            assertEquals(account, assertIs<RepositoryResult.Success<TransactionRecord>>(
                repository.confirm(OccurrenceId(0, 33), assigned, IdempotencyKey("confirmation_key"))).value.transaction.accountId)
            assertEquals(listOf(
                """{"amount":"5.25","currency":"EUR","occurred_on":"2026-11-01","account_id":null,"category_id":null,"description":"Actual"}""",
                """{"amount":"5.25","currency":"EUR","occurred_on":"2026-11-01","account_id":"00000000-0000-0000-0000-000000000009","category_id":null,"description":"Actual"}""",
            ), bodies)
        } finally { client.close(); engine.close() }
    }

    @Test fun skipAndPostponeUseOriginalActionKeysAndStrictLaterDueDate() = runTest {
        val keys = mutableListOf<String?>()
        val bodies = mutableListOf<String>()
        val (client, engine) = api { request ->
            keys += request.headers["Idempotency-Key"]
            bodies += (request.body as TextContent).text
            val due = if (request.url.encodedPath.endsWith("/skip")) "2026-02-05" else "2026-02-06"
            val status = if (request.url.encodedPath.endsWith("/skip")) "skipped" else "planned"
            respond("""{"id":"00000000-0000-0000-0000-000000000021","scheduled_on":"2026-01-31","due_on":"$due","state":"$status"}""", HttpStatusCode.OK)
        }
        try {
            val repository = HttpRecurrenceRepository(client, context)
            val skipped = repository.skip(OccurrenceId(0, 33), IdempotencyKey("skip_key"))
            assertEquals(OccurrenceState.SKIPPED, assertIs<RepositoryResult.Success<OccurrenceActionResult>>(skipped).value.state)
            val postponed = repository.postpone(OccurrenceId(0, 33), BusinessDate(2026, 2, 6), IdempotencyKey("postpone_key"))
            assertEquals(BusinessDate(2026, 2, 6), assertIs<RepositoryResult.Success<OccurrenceActionResult>>(postponed).value.dueOn)
            assertEquals(listOf<String?>("skip_key", "postpone_key"), keys)
            assertEquals(listOf("{}", "{\"due_on\":\"2026-02-06\"}"), bodies)
        } finally { client.close(); engine.close() }
    }
}
