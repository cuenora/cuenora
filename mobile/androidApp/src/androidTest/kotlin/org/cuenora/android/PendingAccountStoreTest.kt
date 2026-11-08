package org.cuenora.android

import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.*
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Test
import java.util.UUID

class PendingAccountStoreTest {
    @Test fun encryptedJournalSurvivesRecreationAndSeparatesOwnersAndOrigins() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "synthetic-onboarding-${UUID.randomUUID()}"
        val owner = OwnerId(0, 7); val other = OwnerId(0, 8)
        val store = AndroidPendingAccountStore(context, namespace)
        val pending = PendingAccount(owner, CreateAccountCommand(AccountName("Synthetic daily"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("synthetic-key"))
        try {
            assertNull(store.read(owner)); store.save(pending)
            val restored = AndroidPendingAccountStore(context, namespace).read(owner)!!
            assertEquals(pending.encode(), restored.encode())
            assertFalse(restored.toString().contains("Synthetic daily"))
            assertNull(store.read(other))
            assertNull(AndroidPendingAccountStore(context, namespace + "-other").read(owner))
            store.save(pending)
            val different = PendingAccount(owner, CreateAccountCommand(AccountName("Other"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("different-key"))
            try { store.save(different); fail("Must not overwrite an unresolved operation") }
            catch (_: PendingAccountStoreException) { }
            assertEquals(pending.encode(), store.read(owner)!!.encode())
            store.clear(owner); assertNull(store.read(owner))
        } finally { store.clear(owner) }
    }
}
