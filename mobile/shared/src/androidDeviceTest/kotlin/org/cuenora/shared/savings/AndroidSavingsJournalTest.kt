package org.cuenora.shared.savings

import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidSavingsJournalTest {
    private val context: Context get() = InstrumentationRegistry.getInstrumentation().targetContext
    @Test fun journalEncryptsBindsAndProtectsOriginalReceiptAndConfiguration() = runBlocking {
        val origin = "https://savings-test.invalid"
        val owner = OwnerId(11, 12)
        val journal = AndroidSavingsJournal(context, origin)
        journal.clear(owner, origin)
        try {
            val receipt = PendingSavings(owner, origin, SavingsAction.Receipt(AccountId(0, 3), "12.34", Currency.SEK,
                "2026-11-10", "private synthetic bank credit"), IdempotencyKey("original_interest"))
            journal.save(owner, origin, receipt)
            val file = context.noBackupFilesDir.listFiles()!!.single { it.name.startsWith("savings-") && it.extension == "record" }
            assertFalse(file.readBytes().decodeToString().contains("private synthetic bank credit"))
            assertEquals(receipt.encode(), AndroidSavingsJournal(context, origin).read(owner, origin)?.encode())
            assertNull(AndroidSavingsJournal(context, "https://other.invalid").read(owner, "https://other.invalid"))
            assertNull(journal.read(OwnerId(11, 13), origin))
            val replacement = PendingSavings(owner, origin, SavingsAction.Rate(AccountId(0, 3), SavingsRateCommand(BusinessDate(2099, 1, 1), "3")), null)
            var rejected = false
            try { journal.save(owner, origin, replacement) } catch (_: SavingsJournalException) { rejected = true }
            assertTrue(rejected)
            assertEquals(receipt.encode(), journal.read(owner, origin)?.encode())
            journal.clear(owner, origin); journal.save(owner, origin, replacement)
            assertEquals(replacement.encode(), journal.read(owner, origin)?.encode())
            val bytes = file.readBytes(); bytes[bytes.lastIndex] = (bytes.last().toInt() xor 1).toByte(); file.writeBytes(bytes)
            rejected = false
            try { journal.read(owner, origin) } catch (_: SavingsJournalException) { rejected = true }
            assertTrue(rejected)
        } finally { journal.clear(owner, origin) }
        assertNull(journal.read(owner, origin))
    }
}
