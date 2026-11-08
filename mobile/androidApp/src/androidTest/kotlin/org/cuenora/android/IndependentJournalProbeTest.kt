package org.cuenora.android
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.*
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test
import java.util.UUID

class IndependentJournalProbeTest {
    @Test fun ciphertextTamperingAndOwnerTransplantFailClosedWithoutOverwrite() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = context.noBackupFilesDir
        fun files() = dir.listFiles()!!.filter { it.name.startsWith("pending-account-") }.toSet()
        val store = AndroidPendingAccountStore(context,"independent-${UUID.randomUUID()}")
        val a = OwnerId(0,71); val b = OwnerId(0,72)
        fun record(owner: OwnerId) = PendingAccount(owner,CreateAccountCommand(AccountName("Secret synthetic account"),AccountType.NORMAL,Currency.SEK),IdempotencyKey("synthetic-secret-key"))
        val previous = files(); store.save(record(a)); val afile = (files()-previous).single(); val original = afile.readBytes()
        assertFalse(original.toString(Charsets.UTF_8).contains("Secret synthetic account"))
        assertFalse(original.toString(Charsets.UTF_8).contains("synthetic-secret-key"))
        val beforeB = files(); store.save(record(b)); val bfile = (files()-beforeB).single()
        bfile.writeBytes(original)
        try { store.read(b); fail("Owner transplant must fail") } catch (_:PendingAccountStoreException) {}
        assertArrayEquals(original,afile.readBytes())
        val corrupt = original.copyOf(); corrupt[corrupt.lastIndex] = (corrupt.last().toInt() xor 1).toByte(); afile.writeBytes(corrupt)
        try { store.read(a); fail("Tampered ciphertext must fail") } catch (_:PendingAccountStoreException) {}
        try { store.save(record(a)); fail("Corrupted unresolved record must block new creation") } catch (_:PendingAccountStoreException) {}
        assertArrayEquals(corrupt,afile.readBytes()); store.clear(a); store.clear(b)
    }
}
