package org.cuenora.android

import org.cuenora.shared.history.AndroidExpenseEditJournal
import org.cuenora.shared.history.PendingExpenseEdit

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.domain.TransactionId
import org.cuenora.shared.repository.IdempotencyKey
import org.cuenora.shared.repository.PatchField
import org.cuenora.shared.repository.TransactionPatch
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidExpenseEditJournalRecoveryTest {
    @Test fun firstSaveInterruptedBeforeAtomicCommitDoesNotBlockNextEdit() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val namespace = "expense-edit-recovery-${System.nanoTime()}"
        val owner = OwnerId(31, 32)
        val uncommitted = File(context.noBackupFilesDir, "pending-expense-edit-${digest("$namespace|31|32")}.record.new")
        val journal = AndroidExpenseEditJournal(context, namespace)
        try {
            val partial = byteArrayOf(1, 0, 2)
            uncommitted.writeBytes(partial)
            assertArrayEquals(partial, uncommitted.readBytes())
            assertNull(journal.read(owner))

            val next = PendingExpenseEdit(owner, TransactionId(41, 42), IdempotencyKey("fresh-edit-key"),
                TransactionPatch(description = PatchField.Value("Fresh correction")))
            journal.save(next)
            assertEquals(next, AndroidExpenseEditJournal(context, namespace).read(owner))
        } finally {
            journal.clear(owner)
            KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
                .deleteEntry("cuenora.pending-expense-edit.${digest(namespace)}")
        }
    }

    private fun digest(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray()).joinToString("") { "%02x".format(it) }
}
