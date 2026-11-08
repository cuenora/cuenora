package org.cuenora.shared.recording

import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertFailsWith

class PendingTransactionTest {
    private val pending = PendingTransaction(
        OwnerId(0, 7),
        TransactionCreateCommand.Income(
            Money(1234, Currency.SEK), BusinessDate(2026, 11, 8),
            AccountId(0, 9), CategoryId(0, 11), "Salary",
        ),
        IdempotencyKey("original-key"),
    )

    @Test fun journalRoundTripsExactCommandAndRedactsItsDisplay() {
        val restored = PendingTransaction.decode(pending.encode())
        assertEquals(pending.owner, restored.owner)
        assertEquals(pending.key, restored.key)
        assertEquals(pending.command, restored.command)
        assertFalse(restored.toString().contains("Salary"))
        assertFalse(restored.toString().contains("original-key"))
    }

    @Test fun journalRejectsUnsupportedKindsAndInvalidAmountsDatesAndCategories() {
        val encoded = pending.encode()
        assertFailsWith<IllegalArgumentException> { PendingTransaction.decode(encoded.replace("income", "transfer")) }
        assertFailsWith<IllegalArgumentException> { PendingTransaction.decode(encoded.replace("12.34", "0.00")) }
        assertFailsWith<IllegalArgumentException> { PendingTransaction.decode(encoded.replace("2026-11-08", "2026-02-30")) }
        assertFailsWith<IllegalArgumentException> {
            PendingTransaction.decode(encoded.replace("00000000-0000-0000-0000-00000000000b", "not-a-uuid"))
        }
    }
}
