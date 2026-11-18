package org.cuenora.shared.history

import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiError
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import kotlin.test.*

class ExpenseEditSessionTest {
    @Test fun uncertainWriteReplaysOriginalPatchAndKeyAfterRecreation() = runTest {
        val owner = OwnerId(0, 1)
        val target = TransactionId(0, 2)
        val patch = TransactionPatch(description = PatchField.Value("Corrected"))
        val journal = MemoryJournal()
        val sent = mutableListOf<PendingExpenseEdit>()
        val write: suspend (PendingExpenseEdit) -> RepositoryResult<TransactionAcknowledgement> = {
            sent += it
            if (sent.size == 1) RepositoryResult.Failure(ApiFailure.Http(503, null, null, null))
            else RepositoryResult.Success(TransactionAcknowledgement(target, TransactionState.POSTED))
        }
        val first = ExpenseEditSession({ owner }, write, journal) { IdempotencyKey("original-key") }
        assertEquals(ExpenseEditResult.Pending, first.save(target, patch))
        val restarted = ExpenseEditSession({ owner }, write, journal) { error("Retry must not allocate a key") }
        assertTrue(restarted.hasPending())
        assertEquals(ExpenseEditResult.Saved, restarted.retry())
        assertEquals(listOf(sent.first(), sent.first()), sent)
        assertNull(journal.read(owner))
    }

    @Test fun pendingPayloadRoundTripsClearsAndExactValues() {
        val pending = PendingExpenseEdit(OwnerId(0, 1), TransactionId(0, 2), IdempotencyKey("original-key"),
            TransactionPatch(amount = PatchField.Value(DecimalAmount("92233720368547758.07")),
                occurredOn = PatchField.Value(BusinessDate(2024, 2, 29)), categoryId = PatchField.Null,
                description = PatchField.Value("Description | with quotes \"")))
        assertEquals(pending, PendingExpenseEdit.decode(pending.encode()))
    }

    @Test fun definiteVoidRejectionDuringRecoveryUnblocksOtherEdits() = runTest {
        val owner = OwnerId(0, 7)
        val journal = MemoryJournal()
        var attempts = 0
        val session = ExpenseEditSession({ owner }, { pending ->
            when (++attempts) {
                1 -> RepositoryResult.Failure(ApiFailure.Http(503, null, null, null))
                2 -> RepositoryResult.Failure(ApiFailure.Http(409,
                    ApiError("transaction_voided", "A voided transaction cannot be edited.", null), null, null))
                else -> RepositoryResult.Success(TransactionAcknowledgement(pending.target, TransactionState.POSTED))
            }
        }, journal) { IdempotencyKey("probe-key-$attempts") }
        val patch = TransactionPatch(description = PatchField.Value("Correction"))
        assertEquals(ExpenseEditResult.Pending, session.save(TransactionId(0, 7), patch))
        assertNotNull(journal.read(owner))
        assertEquals(ExpenseEditResult.Rejected, session.retry())
        assertNull(journal.read(owner))
        assertEquals(ExpenseEditResult.Saved, session.save(TransactionId(0, 8), patch))
    }

    private class MemoryJournal : ExpenseEditJournal {
        private val records = mutableMapOf<OwnerId, PendingExpenseEdit>()
        override suspend fun read(owner: OwnerId) = records[owner]
        override suspend fun save(pending: PendingExpenseEdit) { records[pending.owner] = pending }
        override suspend fun clear(owner: OwnerId) { records.remove(owner) }
    }
}
