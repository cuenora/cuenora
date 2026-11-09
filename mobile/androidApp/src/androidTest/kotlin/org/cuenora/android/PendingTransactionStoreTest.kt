package org.cuenora.android

import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.*
import org.cuenora.shared.recording.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test
import java.util.UUID

class PendingTransactionStoreTest {
    @Test fun encryptedTransferRetainsBothAmountsQuoteAndOriginalKeyAcrossRecreation() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "synthetic-transfer-${UUID.randomUUID()}"
        val owner = OwnerId(0, 7)
        val store = AndroidPendingTransactionStore(context, namespace)
        val original = PendingTransaction(owner, TransactionCreateCommand.Transfer(Money(20000, Currency.SEK), AccountId(0, 1),
            Money(1795, Currency.EUR), AccountId(0, 2), BusinessDate(2026, 11, 9), "000.0900", "Synthetic"), IdempotencyKey("original-transfer"))
        try {
            store.save(original)
            val recreated = AndroidPendingTransactionStore(context, namespace)
            assertEquals(original.encode(), recreated.read(owner)!!.encode())
            assertNull(recreated.read(OwnerId(0, 8)))
            assertNull(AndroidPendingTransactionStore(context, "$namespace-other").read(owner))
            try { recreated.save(PendingTransaction(owner, (original.command as TransactionCreateCommand.Transfer).copy(receivedAmount = Money(1800, Currency.EUR)), original.key))
                fail("Cannot replace the actual amount of an unresolved transfer")
            } catch (_: PendingTransactionStoreException) { }
            assertEquals(original.encode(), recreated.read(owner)!!.encode())
        } finally { store.clear(owner) }
    }
    @Test fun encryptedJournalSurvivesRecreationAndDoesNotMixOwnerOrOrigin() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "synthetic-recording-${UUID.randomUUID()}"
        val owner = OwnerId(0, 7); val other = OwnerId(0, 8)
        val store = AndroidPendingTransactionStore(context, namespace)
        val pending = PendingTransaction(owner,
            TransactionCreateCommand.Expense(Money(1234, Currency.SEK), BusinessDate(2026, 11, 8), description = "Lunch"),
            IdempotencyKey("original-key"))
        try {
            assertNull(store.read(owner)); store.save(pending)
            val restored = AndroidPendingTransactionStore(context, namespace).read(owner)!!
            assertEquals(pending.encode(), restored.encode())
            assertFalse(restored.toString().contains("Lunch"))
            assertNull(store.read(other))
            assertNull(AndroidPendingTransactionStore(context, namespace + "-other").read(owner))
            val different = PendingTransaction(owner,
                TransactionCreateCommand.Income(Money(500, Currency.SEK), BusinessDate(2026, 11, 8)),
                IdempotencyKey("another-key"))
            try { store.save(different); fail("Must not overwrite an unresolved operation") }
            catch (_: PendingTransactionStoreException) { }
            assertEquals(pending.encode(), store.read(owner)!!.encode())
            store.clear(owner); assertNull(store.read(owner))
        } finally { store.clear(owner) }
    }
}
