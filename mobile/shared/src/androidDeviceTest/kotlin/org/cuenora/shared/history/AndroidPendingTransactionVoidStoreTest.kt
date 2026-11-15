package org.cuenora.shared.history

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import android.content.Context
import kotlinx.coroutines.runBlocking
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.domain.TransactionId
import org.cuenora.shared.repository.IdempotencyKey
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidPendingTransactionVoidStoreTest {
    @Test fun encryptedOperationSurvivesRecreationAndStaysOwnerAndOriginBound() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val namespace = "void-store-test-${System.nanoTime()}"
        val owner = OwnerId(31, 32)
        val pending = PendingTransactionVoid(owner, TransactionId(41, 42), IdempotencyKey("original-key-123"))
        AndroidPendingTransactionVoidStore(context, namespace).save(pending)

        val recreated = AndroidPendingTransactionVoidStore(context, namespace)
        assertEquals(pending, recreated.read(owner))
        assertEquals(null, recreated.read(OwnerId(31, 33)))
        assertEquals(null, AndroidPendingTransactionVoidStore(context, "$namespace-other").read(owner))
        recreated.clear(owner)
        assertEquals(null, recreated.read(owner))
    }
}
