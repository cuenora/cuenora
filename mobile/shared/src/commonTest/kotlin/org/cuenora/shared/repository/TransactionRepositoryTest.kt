package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlin.test.*

class TransactionRepositoryTest {
    private val owner = OwnerId(1, 2)
    private val token = "fixture-token"
    private val key = IdempotencyKey("logical_1")
    private val id = transactionId("00000000-0000-0000-0000-000000000001")
    private val a = accountId("00000000-0000-0000-0000-000000000002")
    private val b = accountId("00000000-0000-0000-0000-000000000003")
    private val category = categoryId("00000000-0000-0000-0000-000000000004")
    private val date = BusinessDate(2024, 2, 29)
    private val context get() = RepositoryContext(owner, token)

    @Test fun listEncodesFiltersAndMapsAccountlessTransferOpeningAndVoidedRecords() = runTest {
        val response = """{"transactions":[
          {"id":"00000000-0000-0000-0000-000000000001","kind":"expense","state":"voided","amount":"12.34","currency":"SEK","occurred_on":"2024-02-29","account_id":null,"category_id":null,"description":null,"destination_account_id":null,"received_amount":null,"received_currency":null},
          {"id":"00000000-0000-0000-0000-000000000002","kind":"transfer","state":"voided","amount":"92233720368547758.07","currency":"SEK","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null,"destination_account_id":"00000000-0000-0000-0000-000000000003","received_amount":"1.23","received_currency":"EUR","quoted_rate":"0.000000000001","rate_comparison":{"indicative_received_amount":"0.01","difference":"1.22","mismatch":true}},
          {"id":"00000000-0000-0000-0000-000000000005","kind":"opening_balance","state":"posted","amount":"0.00","currency":"USD","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null,"destination_account_id":null,"received_amount":null,"received_currency":null}
        ],"next_cursor":"opaque / + cursor"}"""
        var capturedMethod: HttpMethod? = null
        var capturedUrl = ""
        var authorization: String? = null
        val engine = MockEngine { request ->
            capturedMethod = request.method
            capturedUrl = request.url.encodedPathAndQuery
            authorization = request.headers[HttpHeaders.Authorization]
            respond(response, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        val page = assertIs<RepositoryResult.Success<Page<TransactionRecord>>>(repo.list(
            TransactionQuery(limit = 7, cursor = "opaque / + cursor", dateFrom = date,
                dateTo = date, categoryId = category, account = TransactionAccountFilter.Accountless)
        )).value
        assertEquals(3, page.items.size)
        assertEquals(TransactionKind.EXPENSE, page.items[0].transaction.kind)
        assertNull(page.items[0].transaction.accountId)
        assertEquals(TransactionState.VOIDED, page.items[0].transaction.state)
        assertEquals(Money(Long.MAX_VALUE, Currency.SEK), page.items[1].transaction.amount)
        assertEquals(Money(123, Currency.EUR), page.items[1].transaction.receivedAmount)
        assertEquals("0.000000000001", page.items[1].quotedRate)
        assertEquals(Money(122, Currency.EUR), page.items[1].rateComparison!!.difference)
        assertEquals(TransactionKind.OPENING_BALANCE, page.items[2].transaction.kind)
        assertEquals("opaque / + cursor", page.nextCursor)
        assertEquals(HttpMethod.Get, capturedMethod)
        assertTrue(capturedUrl.contains("limit=7"))
        assertTrue(capturedUrl.contains("account_id=none"))
        assertTrue(capturedUrl.contains("cursor=opaque+%2F+%2B+cursor"))
        assertEquals("Bearer $token", authorization)
        api.close()
    }

    @Test fun listMapsAnOwnedAccountFilterAndRejectsMissingRequiredTransferFields() = runTest {
        val malformed = """{"transactions":[{"id":"00000000-0000-0000-0000-000000000001","kind":"transfer","state":"posted","amount":"1.00","currency":"SEK","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null,"destination_account_id":"00000000-0000-0000-0000-000000000003","received_amount":null,"received_currency":null}],"next_cursor":null}"""
        var path = ""
        val engine = MockEngine { request -> path = request.url.encodedPathAndQuery; respond(malformed, HttpStatusCode.OK) }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        val result = repo.list(TransactionQuery(account = TransactionAccountFilter.Linked(a)))
        assertTrue(path.contains("account_id=00000000-0000-0000-0000-000000000002"))
        assertIs<RepositoryResult.Failure>(result)
        api.close()
    }

    @Test fun createAllFourCommandKindsPreservesExactAmountsAndOptionalQuote() = runTest {
        val bodies = mutableListOf<JsonObject>()
        val responses = listOf(
            """{"id":"00000000-0000-0000-0000-000000000001","kind":"income","state":"posted","amount":"92233720368547758.07","currency":"SEK","occurred_on":"2024-02-29","account_id":null,"category_id":null,"description":null}""",
            """{"id":"00000000-0000-0000-0000-000000000001","kind":"expense","state":"posted","amount":"1.20","currency":"EUR","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":"00000000-0000-0000-0000-000000000004","description":"cost"}""",
            """{"id":"00000000-0000-0000-0000-000000000001","kind":"interest_income","state":"posted","amount":"2.00","currency":"USD","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null}""",
            """{"id":"00000000-0000-0000-0000-000000000001","kind":"transfer","state":"posted","amount":"1.00","currency":"SEK","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","destination_account_id":"00000000-0000-0000-0000-000000000003","received_amount":"2.00","received_currency":"EUR","category_id":null,"description":null,"quoted_rate":"2.000000000001","rate_comparison":{"indicative_received_amount":"2.00","difference":"0.00","mismatch":false}}"""
        )
        var index = 0
        val engine = MockEngine { request ->
            assertEquals(HttpMethod.Post, request.method)
            assertEquals("Bearer $token", request.headers[HttpHeaders.Authorization])
            assertEquals("logical_1", request.headers["Idempotency-Key"])
            bodies += Json.parseToJsonElement(request.body.toByteArray().decodeToString()).jsonObject
            respond(responses[index++], HttpStatusCode.Created)
        }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        val creates = listOf(
            TransactionCreateCommand.Income(Money(Long.MAX_VALUE, Currency.SEK), date),
            TransactionCreateCommand.Expense(Money(120, Currency.EUR), date, a, category, "cost"),
            TransactionCreateCommand.InterestIncome(Money(200, Currency.USD), date, a),
            TransactionCreateCommand.Transfer(Money(100, Currency.SEK), a, Money(200, Currency.EUR), b, date, "2.000000000001")
        )
        val created = creates.map { command -> assertIs<RepositoryResult.Success<TransactionRecord>>(repo.create(command, key)).value }
        assertEquals(listOf("income", "expense", "interest_income", "transfer"), bodies.map { it["kind"]!!.jsonPrimitive.content })
        assertEquals("92233720368547758.07", bodies.first()["amount"]!!.jsonPrimitive.content)
        assertEquals(JsonNull, bodies.first()["account_id"])
        assertEquals("2.000000000001", created.last().quotedRate)
        assertEquals(Money(200, Currency.EUR), created.last().transaction.receivedAmount)
        assertEquals(Money(200, Currency.EUR), created.last().rateComparison!!.indicativeReceivedAmount)
        assertFalse(created.last().rateComparison!!.mismatch)
        assertFalse(bodies.first().containsKey("destination_account_id"))
        api.close()
    }

    @Test fun editSendsOnlyPresentFieldsAndReturnsAcknowledgementAndVoidSendsEmptyObject() = runTest {
        val requests = mutableListOf<Pair<HttpMethod, JsonObject>>()
        val engine = MockEngine { request ->
            val body = Json.parseToJsonElement(request.body.toByteArray().decodeToString()).jsonObject
            requests += request.method to body
            val state = if (request.method == HttpMethod.Patch) "posted" else "voided"
            respond("""{"id":"00000000-0000-0000-0000-000000000001","state":"$state"}""", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        val patch = TransactionPatch(
            amount = PatchField.Value(DecimalAmount("2.50")),
            currency = PatchField.Absent,
            occurredOn = PatchField.Value(date),
            accountId = PatchField.Null,
            receivedAmount = PatchField.Value(DecimalAmount("3.00")),
            description = PatchField.Value("revised")
        )
        val ack = assertIs<RepositoryResult.Success<TransactionAcknowledgement>>(repo.edit(id, patch, key)).value
        assertEquals(id, ack.id)
        assertEquals(TransactionState.POSTED, ack.state)
        assertEquals("2.50", requests[0].second["amount"]!!.jsonPrimitive.content)
        assertFalse(requests[0].second.containsKey("currency"))
        assertEquals("2024-02-29", requests[0].second["occurred_on"]!!.jsonPrimitive.content)
        assertEquals(JsonNull, requests[0].second["account_id"])
        assertEquals("3.00", requests[0].second["received_amount"]!!.jsonPrimitive.content)
        assertFalse(requests[0].second.containsKey("quoted_rate"))
        assertEquals("logical_1", engine.requestHistory[0].headers["Idempotency-Key"])
        val void = assertIs<RepositoryResult.Success<TransactionAcknowledgement>>(repo.void(id, key)).value
        assertEquals(TransactionState.VOIDED, void.state)
        assertEquals(HttpMethod.Post, requests[1].first)
        assertEquals(JsonObject(emptyMap()), requests[1].second)
        api.close()
    }

    @Test fun failuresAndMalformedOrUnknownResponsesRemainTypedFailuresWithoutRetry() = runTest {
        var calls = 0
        val engine = MockEngine { _ -> calls++; respond("not json", HttpStatusCode.OK) }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        assertIs<RepositoryResult.Failure>(repo.list())
        assertEquals(1, calls)
        val badEngine = MockEngine { _ -> calls++; respond("""{"id":"00000000-0000-0000-0000-000000000001","state":"unknown"}""", HttpStatusCode.OK) }
        val badApi = ApiClient(badEngine, "http://localhost")
        val badRepo = HttpTransactionRepository(badApi, context)
        assertIs<RepositoryResult.Failure>(badRepo.void(id, key))
        assertEquals(2, calls)
        val http = MockEngine { _ -> respond("{}", HttpStatusCode.Forbidden) }
        val httpApi = ApiClient(http, "http://localhost")
        val httpRepo = HttpTransactionRepository(httpApi, context)
        val failure = assertIs<RepositoryResult.Failure>(httpRepo.list()).error
        assertEquals(ApiFailure.Http(403, null, null, null), failure)
        api.close()
        badApi.close()
        httpApi.close()
    }

    @Test fun successPayloadMustMatchCreateKindAndMutationTargetAndAction() = runTest {
        val responses = ArrayDeque<String>().apply {
            add("""{"id":"00000000-0000-0000-0000-000000000001","kind":"opening_balance"}""")
            add("""{"id":"00000000-0000-0000-0000-000000000009","state":"posted"}""")
            add("""{"id":"00000000-0000-0000-0000-000000000001","state":"posted"}""")
        }
        var index = 0
        val engine = MockEngine { _ ->
            val status = if (index++ == 0) HttpStatusCode.Created else HttpStatusCode.OK
            respond(responses.removeFirst(), status)
        }
        val api = ApiClient(engine, "http://localhost")
        val repo = HttpTransactionRepository(api, context)
        assertIs<RepositoryResult.Failure>(repo.create(
            TransactionCreateCommand.Income(Money(100, Currency.SEK), date), key
        ))
        assertIs<RepositoryResult.Failure>(repo.edit(
            id, TransactionPatch(description = PatchField.Value("changed")), key
        ))
        assertIs<RepositoryResult.Failure>(repo.void(id, key))
        api.close()
    }

    @Test fun fakeCapturesAllOperationsAndReturnsConfiguredFailuresAndCancellation() = runTest {
        val fake = FakeTransactionRepository()
        val failure = RepositoryResult.Failure(ApiFailure.Timeout)
        fake.listResult = failure
        assertEquals(failure, fake.list(TransactionQuery(limit = 9)))
        fake.createResult = failure
        val command = TransactionCreateCommand.Income(Money(100, Currency.SEK), date)
        assertEquals(failure, fake.create(command, key))
        fake.editResult = failure
        val patch = TransactionPatch(description = PatchField.Null)
        assertFailsWith<IllegalArgumentException> { TransactionPatch(amount = PatchField.Null, description = PatchField.Value("keep")) }
        assertEquals(failure, fake.edit(id, patch, key))
        fake.voidResult = failure
        assertEquals(failure, fake.void(id, key))
        assertEquals(4, fake.calls.size)
        assertIs<TransactionRepositoryCall.List>(fake.calls[0])
        assertEquals(command, assertIs<TransactionRepositoryCall.Create>(fake.calls[1]).command)
        assertEquals(patch, assertIs<TransactionRepositoryCall.Edit>(fake.calls[2]).patch)
        assertEquals(key, assertIs<TransactionRepositoryCall.Void>(fake.calls[3]).key)
        fake.beforeCall = { throw CancellationException("cancelled") }
        assertFailsWith<CancellationException> { fake.list() }
    }
}
