package org.cuenora.shared.session

import android.content.Context
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidSessionStoreTest {
    private val context: Context
        get() = InstrumentationRegistry.getInstrumentation().targetContext

    @Test
    fun readsAbsenceAndFreshInstanceReadsCompleteReplacement() = runBlocking {
        val namespace = namespace("round-trip")
        val first = AndroidSessionStore(context, namespace)
        assertNull(first.read())

        first.replace(credentials("access-one", "refresh-one"))
        val restored = AndroidSessionStore(context, namespace).read()
        assertEquals("access-one", restored?.accessToken)
        assertEquals("refresh-one", restored?.refreshToken)

        first.replace(credentials("access-two", "refresh-two"))
        val replaced = AndroidSessionStore(context, namespace).read()
        assertEquals("access-two", replaced?.accessToken)
        assertEquals("refresh-two", replaced?.refreshToken)
        first.clear()
    }

    @Test
    fun ciphertextIsStoredAndKeystoreKeyIsNotExportable() = runBlocking {
        val namespace = namespace("ciphertext")
        val store = AndroidSessionStore(context, namespace)
        val access = "synthetic-access-secret"
        val refresh = "synthetic-refresh-secret"
        store.replace(credentials(access, refresh))

        val record = File(context.noBackupFilesDir, recordName(namespace)).readBytes()
        val text = record.decodeToString()
        assertTrue(!text.contains(access))
        assertTrue(!text.contains(refresh))
        val key = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            .getKey(keyAlias(namespace), null)
        assertNull(key.encoded)
        store.clear()
    }

    @Test
    fun namespacesAreIsolatedAndClearIsIdempotent() = runBlocking {
        val left = AndroidSessionStore(context, namespace("left"))
        val right = AndroidSessionStore(context, namespace("right"))
        left.replace(credentials("left-access", "left-refresh"))
        right.replace(credentials("right-access", "right-refresh"))

        left.clear()
        left.clear()
        assertNull(left.read())
        assertEquals("right-access", right.read()?.accessToken)
        assertEquals("right-refresh", right.read()?.refreshToken)
        right.clear()
    }

    @Test
    fun clearDeletesKeystoreAliasAndFreshInstanceCanSaveAgain() = runBlocking {
        val namespace = namespace("clear-key-lifecycle")
        val store = AndroidSessionStore(context, namespace)
        val keys = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        try {
            store.replace(credentials("old-access", "old-refresh"))
            assertTrue(keys.containsAlias(keyAlias(namespace)))

            store.clear()
            assertFalse(keys.containsAlias(keyAlias(namespace)))
            assertFalse(File(context.noBackupFilesDir, recordName(namespace)).exists())
            val fresh = AndroidSessionStore(context, namespace)
            assertNull(fresh.read())

            fresh.replace(credentials("new-access", "new-refresh"))
            assertTrue(keys.containsAlias(keyAlias(namespace)))
            val restored = AndroidSessionStore(context, namespace).read()
            assertEquals("new-access", restored?.accessToken)
            assertEquals("new-refresh", restored?.refreshToken)
        } finally {
            store.clear()
        }
    }

    @Test
    fun corruptRecordAndMissingKeyFailClosed() = runBlocking {
        val corruptNamespace = namespace("corrupt")
        val corruptStore = AndroidSessionStore(context, corruptNamespace)
        corruptStore.replace(credentials("secret-a", "secret-r"))
        val recordFile = File(context.noBackupFilesDir, recordName(corruptNamespace))
        val tamperedRecord = recordFile.readBytes().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
        recordFile.writeBytes(tamperedRecord)
        assertEquals(SessionFailure.Corrupt, assertThrows(SessionStoreException::class.java) {
            runBlocking { corruptStore.read() }
        }.reason)
        corruptStore.clear()

        val missingKeyNamespace = namespace("missing-key")
        val missingKeyStore = AndroidSessionStore(context, missingKeyNamespace)
        missingKeyStore.replace(credentials("secret-a", "secret-r"))
        KeyStore.getInstance("AndroidKeyStore").apply { load(null) }.deleteEntry(keyAlias(missingKeyNamespace))
        assertEquals(SessionFailure.Corrupt, assertThrows(SessionStoreException::class.java) {
            runBlocking { missingKeyStore.read() }
        }.reason)
        missingKeyStore.clear()
    }

    @Test
    fun concurrentInstancesNeverExposeMixedCredentialPairs() = runBlocking {
        val namespace = namespace("concurrent")
        val stores = (0 until 4).map { AndroidSessionStore(context, namespace) }
        val pairs = listOf(credentials("access-a", "refresh-a"), credentials("access-b", "refresh-b"))
        stores.first().replace(pairs.first())

        (0 until 40).map { index ->
            async(Dispatchers.Default) {
                val store = stores[index % stores.size]
                if (index % 2 == 0) store.replace(pairs[(index / 2) % pairs.size])
                else {
                    val observed = store.read()!!
                    assertTrue(
                        (observed.accessToken == "access-a" && observed.refreshToken == "refresh-a") ||
                            (observed.accessToken == "access-b" && observed.refreshToken == "refresh-b"),
                    )
                }
            }
        }.awaitAll()
        stores.first().clear()
    }

    private fun credentials(access: String, refresh: String) = SessionCredentials(access, refresh)
    private fun namespace(suffix: String) = "https://session-test.invalid/$suffix"

    private fun digest(namespace: String) = MessageDigest.getInstance("SHA-256")
        .digest(sessionNamespace(namespace).encodeToByteArray())
        .joinToString("") { "%02x".format(it) }

    private fun recordName(namespace: String) = "session-${digest(namespace)}.record"
    private fun keyAlias(namespace: String) = "cuenora.session.${digest(namespace)}"
}
