package org.cuenora.shared.serialization

import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.nullable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.Money
import org.cuenora.shared.domain.ProjectedMinorUnits

class AmountSerializationTest {
    @Test
    fun recordedAmountsRoundTripExactlyForEveryCurrency() {
        for (currency in Currency.entries) {
            val serializer = MoneyAmountSerializer(currency)
            for ((units, decimal) in listOf(
                0L to "0.00", 1L to "0.01", -1L to "-0.01", 12345L to "123.45",
                9007199254740993L to "90071992547409.93",
                Long.MAX_VALUE to "92233720368547758.07",
                -Long.MAX_VALUE to "-92233720368547758.07",
            )) {
                val money = Money(units, currency)
                assertEquals("\"$decimal\"", Json.encodeToString(serializer, money))
                assertEquals(money, Json.decodeFromString(serializer, "\"$decimal\""))
            }
            for ((input, units) in listOf("0001" to 100L, "1.2" to 120L, "-0" to 0L, "-00.00" to 0L)) {
                assertEquals(Money(units, currency), Json.decodeFromString(serializer, "\"$input\""))
            }
        }
    }

    @Test
    fun recordedAmountsRejectMalformedOverprecisionAndOverflow() {
        val serializer = MoneyAmountSerializer(Currency.SEK)
        for (input in listOf("", "-", "+1", " 1", "1 ", ".1", "1.", "1,00", "1e2", "NaN",
            "1.000", "0.001", "١.00", "1.2.3", "--1", "92233720368547758.08",
            "-92233720368547758.08", "999999999999999999999999999999999999")) {
            assertFailsWith<SerializationException> { Json.decodeFromString(serializer, "\"$input\"") }
        }
        assertFailsWith<SerializationException> {
            Json.encodeToString(serializer, Money(100, Currency.EUR))
        }
    }

    @Test
    fun amountsRequireActualJsonStringsIncludingTreeAndLenientDecoding() {
        for (json in listOf(Json, Json { isLenient = true })) {
            for (input in listOf("1.00", "1", "true", "null", "{}", "[]")) {
                assertFailsWith<SerializationException> { json.decodeFromString(MoneyAmountSerializer(Currency.USD), input) }
                assertFailsWith<SerializationException> { json.decodeFromString(ProjectedAmountSerializer, input) }
                assertFailsWith<SerializationException> { json.decodeFromString(ReportAmountSerializer, input) }
            }
            assertFailsWith<SerializationException> {
                json.decodeFromJsonElement(MoneyAmountSerializer(Currency.USD), JsonPrimitive(1))
            }
        }
    }

    @Test
    fun projectedAmountsKeepArbitraryPrecisionAndNullableOpenTail() {
        val digits = "123456789012345678901234567890123456789"
        val payout = ProjectedMinorUnits(digits)
        val wire = "\"1234567890123456789012345678901234567.89\""
        assertEquals(wire, Json.encodeToString(ProjectedAmountSerializer, payout))
        assertEquals(payout, Json.decodeFromString(ProjectedAmountSerializer, wire))
        for ((minor, decimal) in listOf("0" to "0.00", "1" to "0.01", "10" to "0.10", "100" to "1.00")) {
            assertEquals("\"$decimal\"", Json.encodeToString(ProjectedAmountSerializer, ProjectedMinorUnits(minor)))
            assertEquals(ProjectedMinorUnits(minor), Json.decodeFromString(ProjectedAmountSerializer, "\"$decimal\""))
        }
        assertEquals(ProjectedMinorUnits("1"), Json.decodeFromString(ProjectedAmountSerializer, "\"000.01\""))
        for (input in listOf("-0.00", "-1.00", "1", "1.0", "1.000", "1e2", " 1.00")) {
            assertFailsWith<SerializationException> { Json.decodeFromString(ProjectedAmountSerializer, "\"$input\"") }
        }
        assertEquals("null", Json.encodeToString(ProjectedAmountSerializer.nullable, null))
        assertEquals(null, Json.decodeFromString(ProjectedAmountSerializer.nullable, "null"))
    }

    @Test
    fun reportAmountsRemainUnboundedSignedTwoDecimalStrings() {
        for (amount in listOf("0.00", "-0.01", "123456789012345678901234567890.12", "-123456789012345678901234567890.12")) {
            assertEquals("\"$amount\"", Json.encodeToString(ReportAmountSerializer, amount))
            assertEquals(amount, Json.decodeFromString(ReportAmountSerializer, "\"$amount\""))
        }
        for (amount in listOf("1", "1.0", "1.000", "+1.00", "1e2", " 1.00", "NaN")) {
            assertFailsWith<SerializationException> { Json.encodeToString(ReportAmountSerializer, amount) }
            assertFailsWith<SerializationException> { Json.decodeFromString(ReportAmountSerializer, "\"$amount\"") }
        }
    }
}
