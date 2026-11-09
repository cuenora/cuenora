package org.cuenora.shared.recording

import org.cuenora.shared.domain.*
import org.cuenora.shared.validation.TransactionForm
import kotlin.test.*

class TransferRateComparisonTest {
    private val form = TransactionForm(TransactionKind.TRANSFER, "200", Currency.SEK, "2026-11-09",
        accountId = AccountId(0, 1), destinationAccountId = AccountId(0, 2), receivedAmount = "17.95",
        receivedCurrency = Currency.EUR, quotedRate = "0.09")

    @Test fun exactComparisonPreservesActualsAndUsesDestinationMinorUnits() {
        for ((sent, received, quote, indicative, difference) in listOf(
            listOf("200", "18", "0.09", "1800", "0"),
            listOf("200", "17.95", "000.0900", "1800", "-5"),
            listOf("200", "18.01", "0.09", "1800", "1"),
            listOf("1", "0.12", "0.125", "12", "0"),
            listOf("1", "0.13", "0.125", "12", "1"),
            listOf("1", "0.14", "0.135", "14", "0"),
            listOf("1", "0.14", "0.135000000001", "14", "0"),
            listOf("1", "0.13", "0.125000000001", "13", "0"),
            listOf("1", "0.01", "0.000000000001", "0", "1"),
            listOf("92233720368547758.07", "92233720368547758.07", "1", Long.MAX_VALUE.toString(), "0"),
            listOf("92233720368547758.07", "92233.72", "0.000000000001", "9223372", "0"),
        )) {
            val input = form.copy(amount = sent, receivedAmount = received, quotedRate = quote)
            val result = assertNotNull(transferRateComparison(input))
            assertEquals(Money(indicative.toLong(), Currency.EUR), result.indicativeReceivedAmount, input.toString())
            assertEquals(Money(difference.toLong(), Currency.EUR), result.difference)
            assertEquals(difference != "0", result.mismatch)
            assertEquals(received, input.receivedAmount)
        }
        assertNull(transferRateComparison(form.copy(quotedRate = null)))
        assertNull(transferRateComparison(form.copy(kind = TransactionKind.EXPENSE)))
    }

    @Test fun indicativeOverflowAndMalformedQuotesFailWithoutTruncation() {
        for (quote in listOf("100000000000000000000000000000000000000", "0", "0.1234567890123"))
            assertFailsWith<IllegalArgumentException> { transferRateComparison(form.copy(quotedRate = quote)) }
        assertFailsWith<IllegalArgumentException> {
            transferRateComparison(form.copy(amount = "92233720368547758.07", quotedRate = "1.000000000001"))
        }
    }
}
