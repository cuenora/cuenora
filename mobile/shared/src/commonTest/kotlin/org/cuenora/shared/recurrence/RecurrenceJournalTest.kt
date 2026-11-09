package org.cuenora.shared.recurrence

import kotlin.test.Test
import kotlin.test.assertEquals
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.repository.IdempotencyKey

class RecurrenceJournalTest {
    @Test fun pendingCommandRoundTripsWithoutChangingTheOriginalKeyOrPayload() {
        val command = "{\"amount\":\"12.34\",\"due_on\":\"2026-11-10\"}"
        val pending = PendingRecurrence(OwnerId(1, 2), "https://api.example", IdempotencyKey("retry_key"), command)
        val restored = PendingRecurrence.decode(pending.encode())
        assertEquals(pending.encode(), restored.encode())
        assertEquals("PendingRecurrence([REDACTED])", restored.toString())
    }
}
