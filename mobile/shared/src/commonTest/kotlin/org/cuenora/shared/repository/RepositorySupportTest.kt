package org.cuenora.shared.repository

import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlinx.serialization.json.*
import kotlinx.coroutines.CancellationException
import kotlin.test.*

class RepositorySupportTest {
    @Test fun identitiesAndDatesRoundTripWithoutPlatformDependencies() {
        val text = "fedcba98-7654-3210-0123-456789abcdef"
        assertEquals(text, wireId(accountId(text)))
        assertEquals("0001-02-03", wireDate(BusinessDate(1, 2, 3)))
        assertEquals(BusinessDate(2024, 2, 29), businessDate("2024-02-29"))
        assertFailsWith<IllegalArgumentException> { accountId("../other?x=1") }
    }

    @Test fun contextRedactsCredentialsAndKeysRejectHeaderInjection() {
        val context = RepositoryContext(OwnerId(1, 2), "synthetic-token")
        assertFalse(context.toString().contains("synthetic-token"))
        assertEquals("Bearer synthetic-token", context.headers()["Authorization"])
        assertEquals("key_1", context.headers(IdempotencyKey("key_1"))["Idempotency-Key"])
        assertFailsWith<IllegalArgumentException> { IdempotencyKey("bad\r\nheader") }
        assertFailsWith<IllegalArgumentException> { RepositoryContext(OwnerId(1, 2), "bad\nsecret") }
    }

    @Test fun mappingIsExactAndFailuresDoNotRetainBody() {
        val amount = Money(Long.MAX_VALUE, Currency.SEK)
        assertEquals(amount, readMoney(writeMoney(amount), Currency.SEK))
        assertIs<RepositoryResult.Failure>(mapResponse(ApiResult.Success(200, JsonPrimitive(1), null), 200) {
            it.jsonObject
        })
        assertEquals(RepositoryResult.Failure(ApiFailure.Timeout),
            mapResponse(ApiResult.Failure(ApiFailure.Timeout), 200) { it })
        assertIs<RepositoryResult.Failure>(mapResponse(ApiResult.Success(201, JsonObject(emptyMap()), null), 200) { it })
        assertIs<RepositoryResult.Failure>(mapResponse(ApiResult.Success(200, JsonObject(emptyMap()), null), 200) {
            it.jsonObject.string("missing")
        })
        assertFailsWith<CancellationException> {
            mapResponse(ApiResult.Success(200, JsonNull, null), 200) { throw CancellationException() }
        }
    }
}
