package org.cuenora.shared.recurrence

import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.repository.IdempotencyKey
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidRecurrenceJournalTest {
    private val context: Context get() = InstrumentationRegistry.getInstrumentation().targetContext

    @Test fun journalIsEncryptedOwnerAndOriginBoundAndSurvivesFreshInstance() = runBlocking {
        val origin = "https://recurrence-test.invalid"
        val owner = OwnerId(11, 12)
        val journal = AndroidRecurrenceJournal(context, origin)
        val pending = PendingRecurrence(owner, origin, IdempotencyKey("confirm_original"),
            "{\"path\":\"/v1/recurring-occurrences/1/confirm\",\"body\":\"private synthetic amount\"}")
        journal.save(owner, origin, pending)
        val file = context.noBackupFilesDir.listFiles()!!.single { it.name.startsWith("recurrence-") && it.extension == "record" }
        assertTrue(!file.readBytes().decodeToString().contains("private synthetic amount"))
        assertEquals(pending.encode(), AndroidRecurrenceJournal(context, origin).read(owner, origin)?.encode())
        assertNull(AndroidRecurrenceJournal(context, "https://other.invalid").read(owner, "https://other.invalid"))
        assertNull(journal.read(OwnerId(11, 13), origin))
        journal.clear(owner, origin)
        assertNull(journal.read(owner, origin))
    }
}
