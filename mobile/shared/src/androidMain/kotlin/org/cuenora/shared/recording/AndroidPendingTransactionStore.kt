package org.cuenora.shared.recording

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.cuenora.shared.config.BackendConfig
import org.cuenora.shared.domain.OwnerId

/** Encrypted, owner/origin-bound write-ahead journal in no-backup storage. */
class AndroidPendingTransactionStore(context: Context, namespace: String = BackendConfig.BASE_URL) : PendingTransactionStore {
    private val directory = context.applicationContext.noBackupFilesDir
    private val namespace = namespace.also { require(it.isNotBlank() && it.length <= 1024) }
    private fun digest(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray())
        .joinToString("") { "%02x".format(it) }
    private val alias = "cuenora.pending-transaction." + digest(namespace)
    private fun binding(owner: OwnerId) = "$namespace|${owner.highBits}|${owner.lowBits}"
    private fun file(owner: OwnerId) = File(directory, "pending-transaction-${digest(binding(owner))}.record")
    private fun atomic(owner: OwnerId) = AtomicFile(file(owner))

    override suspend fun read(owner: OwnerId): PendingTransaction? = storage { readRecord(owner) }

    private fun readRecord(owner: OwnerId): PendingTransaction? {
        val base = file(owner)
        if (!base.exists() && !File(base.path + ".bak").exists() && !File(base.path + ".new").exists()) return null
        val record = atomic(owner).openRead().use { input ->
            val data = ByteArray(16385)
            var count = 0
            while (count < data.size) {
                val read = input.read(data, count, data.size - count)
                if (read < 0) break
                count += read
            }
            require(count in 29..16384)
            data.copyOf(count)
        }
        require(record[0] == 1.toByte())
        val secret = keyStore().getKey(alias, null) as? SecretKey ?: throw PendingTransactionStoreException()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, secret, GCMParameterSpec(128, record.copyOfRange(1, 13)))
        cipher.updateAAD(binding(owner).toByteArray())
        return PendingTransaction.decode(cipher.doFinal(record.copyOfRange(13, record.size)).decodeToString())
            .also { require(it.owner == owner) }
    }

    override suspend fun save(pending: PendingTransaction) = storage {
        // An unresolved record (including a damaged/unknown operation) is never overwritten.
        readRecord(pending.owner)?.let {
            require(it.encode() == pending.encode())
            return@storage
        }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(binding(pending.owner).toByteArray())
        val encoded = pending.encode().toByteArray()
        require(encoded.size <= 16000)
        val record = byteArrayOf(1) + cipher.iv + cipher.doFinal(encoded)
        val target = atomic(pending.owner)
        val output = target.startWrite()
        try { output.write(record); target.finishWrite(output) }
        catch (failure: Exception) { target.failWrite(output); throw failure }
        require(readRecord(pending.owner)?.encode() == pending.encode())
    }

    override suspend fun clear(owner: OwnerId) = storage {
        atomic(owner).delete()
        val base = file(owner)
        require(!base.exists() && !File(base.path + ".bak").exists() && !File(base.path + ".new").exists())
    }

    private fun keyStore() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private fun key(): SecretKey {
        keyStore().getKey(alias, null)?.let { return it as? SecretKey ?: throw PendingTransactionStoreException() }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256).setRandomizedEncryptionRequired(true).build())
        }.generateKey()
    }

    private suspend fun <T> storage(block: () -> T): T = withContext(Dispatchers.IO) {
        mutex.withLock {
            try { block() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { throw PendingTransactionStoreException() }
        }
    }

    private companion object { val mutex = Mutex() }
}
