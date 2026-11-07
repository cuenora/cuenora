package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlin.test.*

class RepositoryRegressionTest {
    private val id = TransactionId(0, 1)
    private val date = BusinessDate(2024, 2, 29)
    private val key = IdempotencyKey("probe")
    private val context = RepositoryContext(OwnerId(0, 1), "synthetic-token")

    @Test fun createRejectsEachIndependentRecordedFieldMismatch() = runTest {
        val base = Json.parseToJsonElement("""{"id":"00000000-0000-0000-0000-000000000001","kind":"income","state":"posted","amount":"1.00","currency":"SEK","occurred_on":"2024-02-29","account_id":null,"category_id":null,"description":null}""").jsonObject
        val cases = mapOf(
            "amount" to JsonPrimitive("999.00"),
            "currency" to JsonPrimitive("USD"),
            "occurred_on" to JsonPrimitive("2024-02-28"),
            "description" to JsonPrimitive("different"),
            "account_id" to JsonPrimitive("00000000-0000-0000-0000-000000000002"),
            "category_id" to JsonPrimitive("00000000-0000-0000-0000-000000000003"),
        )
        for ((field, value) in cases) {
            val engine = MockEngine {
                respond(JsonObject(base + (field to value)).toString(), HttpStatusCode.Created)
            }
            val api = ApiClient(engine, "http://localhost")
            try {
                val result = HttpTransactionRepository(api, context).create(
                    TransactionCreateCommand.Income(Money(100, Currency.SEK), date), key)
                assertEquals(RepositoryResult.Failure(ApiFailure.InvalidResponse(201, null)), result, field)
                assertEquals(1, engine.requestHistory.size)
            } finally { api.close(); engine.close() }
        }
    }

    @Test fun amountOnlyAndCurrencyOnlyPatchesRemainIndependent() = runTest {
        val bodies = mutableListOf<JsonObject>()
        val engine = MockEngine { request ->
            bodies += Json.parseToJsonElement(request.body.toByteArray().decodeToString()).jsonObject
            respond("""{"id":"00000000-0000-0000-0000-000000000001","state":"posted"}""", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "http://localhost")
        val repository = HttpTransactionRepository(api, context)
        try {
            assertIs<RepositoryResult.Success<TransactionAcknowledgement>>(repository.edit(id,
                TransactionPatch(amount = PatchField.Value(DecimalAmount("2.50")),
                    receivedAmount = PatchField.Value(DecimalAmount("3.00"))), key))
            assertEquals(JsonObject(mapOf("amount" to JsonPrimitive("2.50"),
                "received_amount" to JsonPrimitive("3.00"))), bodies[0])
            assertIs<RepositoryResult.Success<TransactionAcknowledgement>>(repository.edit(id,
                TransactionPatch(currency = PatchField.Value(Currency.EUR),
                    receivedCurrency = PatchField.Value(Currency.USD)), key))
            assertEquals(JsonObject(mapOf("currency" to JsonPrimitive("EUR"),
                "received_currency" to JsonPrimitive("USD"))), bodies[1])
            assertEquals(2, engine.requestHistory.size)
        } finally { api.close(); engine.close() }
    }

    @Test fun decimalPatchFieldsRetainExactTextAndRejectLossyValues() {
        assertEquals("92233720368547758.07", DecimalAmount("92233720368547758.07").value)
        assertEquals("-2.50", DecimalAmount("-2.50").value)
        for (invalid in listOf("1.001", "1e2", "NaN", "92233720368547758.08")) {
            assertFails { DecimalAmount(invalid) }
        }
    }
    @Test fun patchUsesCurrencylessAmountsAndExplicitCurrencyChanges() = runTest {
        var body: JsonObject? = null
        val engine = MockEngine { request ->
            body = Json.parseToJsonElement(request.body.toByteArray().decodeToString()).jsonObject
            respond("""{"id":"00000000-0000-0000-0000-000000000001","state":"posted"}""", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "http://localhost")
        val result = HttpTransactionRepository(api, context).edit(id,
            TransactionPatch(amount=PatchField.Value(DecimalAmount("2.50")), currency=PatchField.Value(Currency.EUR),
                receivedAmount=PatchField.Value(DecimalAmount("3.00")), receivedCurrency=PatchField.Value(Currency.USD)), key)
        assertIs<RepositoryResult.Success<TransactionAcknowledgement>>(result)
        val sent = assertNotNull(body)
        assertEquals(JsonPrimitive("EUR"), sent["currency"])
        assertEquals(JsonPrimitive("USD"), sent["received_currency"])
        assertEquals(JsonPrimitive("2.50"), sent["amount"])
        assertEquals(JsonPrimitive("3.00"), sent["received_amount"])
        api.close()
    }
    @Test fun createMustRejectDifferentRecordedMoneyAndAccount() = runTest {
        val engine = MockEngine { respond("""{"id":"00000000-0000-0000-0000-000000000001","kind":"income","state":"posted","amount":"999.00","currency":"USD","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null}""", HttpStatusCode.Created) }
        val api = ApiClient(engine, "http://localhost")
        assertIs<RepositoryResult.Failure>(HttpTransactionRepository(api, context).create(
            TransactionCreateCommand.Income(Money(100, Currency.SEK), date), key))
        api.close()
    }
    @Test fun transferMustRejectStringInsteadOfBooleanMismatch() = runTest {
        val engine = MockEngine { respond("""{"transactions":[{"id":"00000000-0000-0000-0000-000000000001","kind":"transfer","state":"posted","amount":"1.00","currency":"SEK","occurred_on":"2024-02-29","account_id":"00000000-0000-0000-0000-000000000002","category_id":null,"description":null,"destination_account_id":"00000000-0000-0000-0000-000000000003","received_amount":"2.00","received_currency":"EUR","quoted_rate":"2","rate_comparison":{"indicative_received_amount":"2.00","difference":"0.00","mismatch":"false"}}],"next_cursor":null}""", HttpStatusCode.OK) }
        val api = ApiClient(engine, "http://localhost")
        assertIs<RepositoryResult.Failure>(HttpTransactionRepository(api, context).list())
        api.close()
    }
    @Test fun sharedClientDoesNotLeakSessionHeadersOrOwnerContext() = runTest {
        val headers = mutableListOf<String?>()
        val engine = MockEngine { request ->
            headers += request.headers[HttpHeaders.Authorization]
            respond("""{"accounts":[{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active","balance":"1.00"}],"next_cursor":null}""", HttpStatusCode.OK)
        }
        val api = ApiClient(engine, "http://localhost")
        val one = HttpAccountRepository(api, RepositoryContext(OwnerId(0, 1), "session-one"))
        val two = HttpAccountRepository(api, RepositoryContext(OwnerId(0, 2), "session-two"))
        val first = assertIs<RepositoryResult.Success<Page<AccountWithBalance>>>(one.list())
        val second = assertIs<RepositoryResult.Success<Page<AccountWithBalance>>>(two.list())
        one.list()
        assertEquals(listOf<String?>("Bearer session-one", "Bearer session-two", "Bearer session-one"), headers)
        assertEquals(OwnerId(0,1), first.value.items[0].account.ownerId)
        assertEquals(OwnerId(0,2), second.value.items[0].account.ownerId)
        api.close()
    }
}
