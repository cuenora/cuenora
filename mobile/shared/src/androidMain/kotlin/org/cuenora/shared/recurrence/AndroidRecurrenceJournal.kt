package org.cuenora.shared.recurrence

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
import org.cuenora.shared.domain.OwnerId

/** No-backup, AES-GCM encrypted write-ahead journal bound to backend origin and owner. */
class AndroidRecurrenceJournal(context: Context, private val origin: String) : RecurrenceJournal {
    private val directory = context.applicationContext.noBackupFilesDir
    init { require(origin.isNotBlank() && origin.length <= 1024) }
    private fun digest(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray()).joinToString("") { "%02x".format(it) }
    private val alias = "cuenora.recurrence." + digest(origin)
    private fun binding(owner: OwnerId) = "$origin|${owner.highBits}|${owner.lowBits}"
    private fun file(owner: OwnerId) = File(directory, "recurrence-${digest(binding(owner))}.record")
    private fun atomic(owner: OwnerId) = AtomicFile(file(owner))

    override suspend fun read(owner: OwnerId, origin: String): PendingRecurrence? = storage {
        require(this@AndroidRecurrenceJournal.origin == origin)
        readRecord(owner)
    }
    override suspend fun save(owner: OwnerId, origin: String, pending: PendingRecurrence) = storage {
        require(this@AndroidRecurrenceJournal.origin == origin && pending.origin == origin && pending.owner == owner)
        readRecord(owner)?.let { require(it.encode() == pending.encode()); return@storage }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key()); cipher.updateAAD(binding(owner).toByteArray())
        val plain = pending.encode().toByteArray(); require(plain.size <= 16000)
        val record = byteArrayOf(1) + cipher.iv + cipher.doFinal(plain)
        val target = atomic(owner); val output = target.startWrite()
        try { output.write(record); target.finishWrite(output) } catch (failure: Exception) { target.failWrite(output); throw failure }
        require(readRecord(owner)?.encode() == pending.encode())
    }
    override suspend fun clear(owner: OwnerId, origin: String) = storage {
        require(this@AndroidRecurrenceJournal.origin == origin)
        atomic(owner).delete()
        val base = file(owner)
        require(!base.exists() && !File(base.path + ".bak").exists() && !File(base.path + ".new").exists())
    }
    private fun readRecord(owner: OwnerId): PendingRecurrence? {
        val base = file(owner)
        if (!base.exists() && !File(base.path + ".bak").exists() && !File(base.path + ".new").exists()) return null
        val record = atomic(owner).openRead().use { input ->
            val data = ByteArray(16385); var count = 0
            while (count < data.size) { val read = input.read(data, count, data.size - count); if (read < 0) break; count += read }
            require(count in 29..16384); data.copyOf(count)
        }
        require(record[0] == 1.toByte())
        val secret = keyStore().getKey(alias, null) as? SecretKey ?: throw RecurrenceJournalException()
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, secret, GCMParameterSpec(128, record.copyOfRange(1, 13)))
        cipher.updateAAD(binding(owner).toByteArray())
        return PendingRecurrence.decode(cipher.doFinal(record.copyOfRange(13, record.size)).decodeToString())
            .also { require(it.owner == owner && it.origin == origin) }
    }
    private fun key(): SecretKey {
        keyStore().getKey(alias, null)?.let { return it as? SecretKey ?: throw RecurrenceJournalException() }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256).setRandomizedEncryptionRequired(true).build())
        }.generateKey()
    }
    private fun keyStore() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private suspend fun <T> storage(block: () -> T): T = withContext(Dispatchers.IO) {
        mutex.withLock { try { block() } catch (cancelled: CancellationException) { throw cancelled } catch (_: Exception) { throw RecurrenceJournalException() } }
    }
    private companion object { val mutex = Mutex() }
}
