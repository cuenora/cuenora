package org.cuenora.shared.session

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileInputStream
import java.io.IOException
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.AEADBadTagException
import javax.crypto.BadPaddingException
import javax.crypto.Cipher
import javax.crypto.IllegalBlockSizeException
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import org.cuenora.shared.config.BackendConfig

/**
 * Stores one encrypted access/refresh pair in this app's no-backup storage.
 *
 * Operations are serialized with every other [SessionStore] in this process and
 * run off the caller thread on a shared background dispatcher. Android Keystore protects the AES key; the
 * platform does not promise that every device backs it with secure hardware.
 * This store does not coordinate across processes, zeroize in-memory secrets, or
 * provide a biometric gate. A native failure can leave a write outcome ambiguous.
 */
class AndroidSessionStore @Throws(IllegalArgumentException::class) constructor(
    context: Context,
    namespace: String = BackendConfig.BASE_URL,
) : SessionStore {
    private val appContext = context.applicationContext
    private val namespace = sessionNamespace(namespace)
    private val digest = MessageDigest.getInstance("SHA-256")
        .digest(this.namespace.encodeToByteArray())
        .joinToString("") { "%02x".format(it) }
    private val keyAlias = "$KEY_ALIAS_PREFIX$digest"
    private val baseFile = File(appContext.noBackupFilesDir, "$RECORD_PREFIX$digest$RECORD_SUFFIX")
    private val atomicFile = AtomicFile(baseFile)
    private val backupFile = File(baseFile.path + ".bak")
    private val newFile = File(baseFile.path + ".new")

    override suspend fun read(): SessionCredentials? = sessionOperation {
        val record = readRecordOrNull() ?: return@sessionOperation null
        val key = existingKey() ?: corrupt()
        val (iv, encrypted) = parseRecord(record)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        try {
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_BITS, iv))
            cipher.updateAAD(associatedData())
            SessionCodec.decode(cipher.doFinal(encrypted))
        } catch (_: AEADBadTagException) {
            corrupt()
        } catch (_: BadPaddingException) {
            corrupt()
        } catch (_: IllegalBlockSizeException) {
            corrupt()
        } catch (failure: SessionStoreException) {
            throw failure
        } catch (_: GeneralSecurityException) {
            unavailable()
        }
    }

    override suspend fun replace(credentials: SessionCredentials) = sessionOperation {
        val previous = readRecordOrNull()
        val key = loadOrCreateKey()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        cipher.updateAAD(associatedData())
        val encrypted = cipher.doFinal(SessionCodec.encode(credentials))
        val record = encodeRecord(cipher.iv, encrypted)

        try {
            writeRecord(record)
            if (!readRecordOrNull().contentEquals(record)) {
                throw IOException("Session record readback did not match")
            }
        } catch (failure: Exception) {
            restorePrevious(previous)
            if (failure is SessionStoreException) throw failure
            unavailable()
        }
    }

    override suspend fun clear() = sessionOperation {
        atomicFile.delete()
        if (hasRecordArtifacts()) unavailable()

        val keyStore = keyStore()
        if (keyStore.containsAlias(keyAlias)) keyStore.deleteEntry(keyAlias)
        if (keyStore.containsAlias(keyAlias)) unavailable()
    }

    private fun readRecordOrNull(): ByteArray? {
        if (!baseFile.exists() && !backupFile.exists() && !newFile.exists()) return null
        if (!baseFile.exists() && !backupFile.exists()) corrupt()
        return try {
            atomicFile.openRead().use(::readBounded)
        } catch (_: java.io.FileNotFoundException) {
            corrupt()
        }
    }

    private fun readBounded(input: FileInputStream): ByteArray {
        val output = ByteArrayOutputStream(minOf(MAX_RECORD_BYTES, 128))
        val buffer = ByteArray(1024)
        var total = 0
        while (true) {
            val count = input.read(buffer)
            if (count < 0) break
            total += count
            if (total > MAX_RECORD_BYTES) corrupt()
            output.write(buffer, 0, count)
        }
        return output.toByteArray()
    }

    private fun parseRecord(record: ByteArray): Pair<ByteArray, ByteArray> {
        if (record.size < HEADER_BYTES + GCM_TAG_BYTES || record.size > MAX_RECORD_BYTES) corrupt()
        if (!record.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)) corrupt()
        if (record[MAGIC.size].toInt() and 0xff != FORMAT_VERSION) corrupt()
        val ivSize = record[MAGIC.size + 1].toInt() and 0xff
        if (ivSize != GCM_IV_BYTES) corrupt()
        val encryptedSize = ((record[MAGIC.size + 2].toInt() and 0xff) shl 24) or
            ((record[MAGIC.size + 3].toInt() and 0xff) shl 16) or
            ((record[MAGIC.size + 4].toInt() and 0xff) shl 8) or
            (record[MAGIC.size + 5].toInt() and 0xff)
        if (encryptedSize !in GCM_TAG_BYTES..MAX_ENCRYPTED_BYTES) corrupt()
        if (record.size != HEADER_BYTES + ivSize + encryptedSize) corrupt()
        val ivStart = HEADER_BYTES
        val encryptedStart = ivStart + ivSize
        return record.copyOfRange(ivStart, encryptedStart) to record.copyOfRange(encryptedStart, record.size)
    }

    private fun encodeRecord(iv: ByteArray, encrypted: ByteArray): ByteArray {
        if (iv.size != GCM_IV_BYTES || encrypted.size !in GCM_TAG_BYTES..MAX_ENCRYPTED_BYTES) unavailable()
        return ByteArray(HEADER_BYTES + iv.size + encrypted.size).also { record ->
            MAGIC.copyInto(record)
            record[MAGIC.size] = FORMAT_VERSION.toByte()
            record[MAGIC.size + 1] = iv.size.toByte()
            for (index in 0..3) {
                record[MAGIC.size + 2 + index] = (encrypted.size ushr (24 - index * 8)).toByte()
            }
            iv.copyInto(record, HEADER_BYTES)
            encrypted.copyInto(record, HEADER_BYTES + iv.size)
        }
    }

    private fun associatedData(): ByteArray = byteArrayOf(FORMAT_VERSION.toByte()) + namespace.encodeToByteArray()

    private fun writeRecord(record: ByteArray) {
        val stream = atomicFile.startWrite()
        try {
            stream.write(record)
            atomicFile.finishWrite(stream)
        } catch (failure: Exception) {
            try {
                atomicFile.failWrite(stream)
            } catch (_: Exception) {
                // Preserve the original failure; the caller performs readback/rollback.
            }
            throw failure
        }
    }

    private fun restorePrevious(previous: ByteArray?) {
        try {
            if (previous == null) {
                atomicFile.delete()
                if (hasRecordArtifacts()) unavailable()
            } else {
                writeRecord(previous)
                if (!readRecordOrNull().contentEquals(previous)) unavailable()
            }
        } catch (_: Exception) {
            unavailable()
        }
    }

    private fun hasRecordArtifacts() = baseFile.exists() || backupFile.exists() || newFile.exists()

    private fun loadOrCreateKey(): SecretKey {
        val keyStore = keyStore()
        val found = keyStore.getKey(keyAlias, null)
        if (found != null) return found as? SecretKey ?: unavailable()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEY_STORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                keyAlias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(AES_KEY_BITS)
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        return generator.generateKey()
    }

    private fun existingKey(): SecretKey? {
        val key = keyStore().getKey(keyAlias, null) ?: return null
        return key as? SecretKey ?: unavailable()
    }

    private fun keyStore(): KeyStore = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }

    private fun corrupt(): Nothing = throw SessionStoreException(SessionFailure.Corrupt)
    private fun unavailable(): Nothing = throw SessionStoreException(SessionFailure.Unavailable)

    private companion object {
        const val ANDROID_KEY_STORE = "AndroidKeyStore"
        const val KEY_ALIAS_PREFIX = "cuenora.session."
        const val RECORD_PREFIX = "session-"
        const val RECORD_SUFFIX = ".record"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val AES_KEY_BITS = 256
        const val FORMAT_VERSION = 1
        const val GCM_IV_BYTES = 12
        const val GCM_TAG_BITS = 128
        const val GCM_TAG_BYTES = GCM_TAG_BITS / 8
        val MAGIC = byteArrayOf('C'.code.toByte(), 'N'.code.toByte(), 'S'.code.toByte(), 'R'.code.toByte())
        const val HEADER_BYTES = 10
        const val MAX_ENCRYPTED_BYTES = MAX_SESSION_BYTES + GCM_TAG_BYTES
        const val MAX_RECORD_BYTES = HEADER_BYTES + GCM_IV_BYTES + MAX_ENCRYPTED_BYTES
    }
}
