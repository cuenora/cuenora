@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class, kotlinx.cinterop.BetaInteropApi::class)

package org.cuenora.shared.session

import kotlinx.cinterop.COpaquePointerVar
import kotlinx.cinterop.alloc
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.get
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.reinterpret
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import org.cuenora.shared.config.BackendConfig
import platform.CoreFoundation.CFDataGetBytePtr
import platform.CoreFoundation.CFDataGetLength
import platform.CoreFoundation.CFRelease
import platform.CoreFoundation.CFRetain
import platform.CoreFoundation.CFDictionarySetValue
import platform.CoreFoundation.CFMutableDictionaryRef
import platform.CoreFoundation.kCFBooleanFalse
import platform.CoreFoundation.kCFBooleanTrue
import platform.Security.SecItemAdd
import platform.Security.SecItemCopyMatching
import platform.Security.SecItemDelete
import platform.Security.SecItemUpdate
import platform.Security.errSecDuplicateItem
import platform.Security.errSecItemNotFound
import platform.Security.errSecSuccess
import platform.Security.kSecAttrAccessible
import platform.Security.kSecAttrAccessibleWhenUnlockedThisDeviceOnly
import platform.Security.kSecAttrAccount
import platform.Security.kSecAttrService
import platform.Security.kSecAttrSynchronizable
import platform.Security.kSecClass
import platform.Security.kSecClassGenericPassword
import platform.Security.kSecMatchLimit
import platform.Security.kSecMatchLimitOne
import platform.Security.kSecReturnData
import platform.Security.kSecReturnAttributes
import platform.Security.kSecValueData
import platform.Foundation.NSData
import platform.Foundation.CFBridgingRelease
import platform.Foundation.CFBridgingRetain
import platform.Foundation.NSString
import platform.Foundation.NSNumber
import platform.Foundation.NSMutableDictionary
import platform.Foundation.create
import platform.Foundation.setObject
import platform.posix.memcpy

private const val SESSION_KEYCHAIN_SERVICE = "org.cuenora.session.v1"

/** Stores the complete access/refresh pair as one device-only generic-password item. */
class KeychainSessionStore @Throws(IllegalArgumentException::class) constructor(
    namespace: String = BackendConfig.BASE_URL,
) : SessionStore {
    private val account = sessionNamespace(namespace)

    override suspend fun read(): SessionCredentials? = sessionOperation {
        val result = copyKeychainResult(readQuery())
        if (result == null) {
            null
        } else {
            try {
                val byteCount = CFDataGetLength(result as platform.CoreFoundation.CFDataRef)
                if (byteCount !in 1..MAX_SESSION_BYTES.toLong()) {
                    throw SessionStoreException(SessionFailure.Corrupt)
                }
                val bytes = ByteArray(byteCount.toInt())
                val source = CFDataGetBytePtr(result as platform.CoreFoundation.CFDataRef)
                    ?: throw SessionStoreException(SessionFailure.Corrupt)
                bytes.usePinned { pinned -> memcpy(pinned.addressOf(0), source, byteCount.toULong()) }
                SessionCodec.decode(bytes)
            } finally {
                CFRelease(result)
            }
        }
    }

    override suspend fun replace(credentials: SessionCredentials) = sessionOperation {
        val encoded = SessionCodec.encode(credentials)
        if (encoded.size !in 1..MAX_SESSION_BYTES) {
            throw SessionStoreException(SessionFailure.Corrupt)
        }
        val data = encoded.toNSData()
        val selector = itemSelector()
        val attributes = keychainDictionary(
            kSecValueData to data,
            kSecAttrAccessible to keychainString(kSecAttrAccessibleWhenUnlockedThisDeviceOnly),
            kSecAttrSynchronizable to false,
        )

        val updateStatus = withCFDictionary(selector) { query ->
            withCFDictionary(attributes) { values -> SecItemUpdate(query, values) }
        }
        when (updateStatus) {
            errSecSuccess -> Unit
            errSecItemNotFound -> addOrResolveConcurrentAdd(data)
            else -> throw SessionStoreException(SessionFailure.Unavailable)
        }
    }

    override suspend fun clear() = sessionOperation {
        val status = withCFDictionary(itemSelector()) { query -> SecItemDelete(query) }
        if (status != errSecSuccess && status != errSecItemNotFound) {
            throw SessionStoreException(SessionFailure.Unavailable)
        }
    }

    private fun addOrResolveConcurrentAdd(data: NSData) {
        val item = keychainDictionary(
            kSecClass to keychainString(kSecClassGenericPassword),
            kSecAttrService to SESSION_KEYCHAIN_SERVICE,
            kSecAttrAccount to account,
            kSecValueData to data,
            kSecAttrAccessible to keychainString(kSecAttrAccessibleWhenUnlockedThisDeviceOnly),
            kSecAttrSynchronizable to false,
        )
        val addStatus = withCFDictionary(item) { query -> SecItemAdd(query, null) }
        if (addStatus == errSecSuccess) return
        if (addStatus != errSecDuplicateItem) {
            throw SessionStoreException(SessionFailure.Unavailable)
        }

        val updateStatus = withCFDictionary(itemSelector()) { query ->
            withCFDictionary(
                keychainDictionary(
                kSecValueData to data,
                kSecAttrAccessible to keychainString(kSecAttrAccessibleWhenUnlockedThisDeviceOnly),
                kSecAttrSynchronizable to false,
                ),
            ) { values -> SecItemUpdate(query, values) }
        }
        if (updateStatus != errSecSuccess) {
            throw SessionStoreException(SessionFailure.Unavailable)
        }
    }

    private fun itemSelector() = keychainDictionary(
        kSecClass to keychainString(kSecClassGenericPassword),
        kSecAttrService to SESSION_KEYCHAIN_SERVICE,
        kSecAttrAccount to account,
        kSecAttrSynchronizable to false,
    )

    private fun readQuery() = keychainDictionary(
        kSecClass to keychainString(kSecClassGenericPassword),
        kSecAttrService to SESSION_KEYCHAIN_SERVICE,
        kSecAttrAccount to account,
        kSecAttrSynchronizable to false,
        kSecReturnData to true,
        kSecMatchLimit to keychainString(kSecMatchLimitOne),
    )
}

internal fun keychainString(value: platform.CoreFoundation.CFStringRef?): NSString =
    CFBridgingRelease(CFRetain(value)) as NSString

internal fun keychainDictionary(vararg entries: Pair<platform.CoreFoundation.CFStringRef?, Any?>): NSMutableDictionary =
    NSMutableDictionary().apply {
        entries.forEach { (key, value) ->
            val nativeValue = if (value is Boolean) NSNumber(bool = value) else value
            setObject(nativeValue, forKey = keychainString(key))
        }
    }

internal inline fun <T> withCFDictionary(dictionary: NSMutableDictionary, block: (platform.CoreFoundation.CFDictionaryRef) -> T): T {
    val retained: CFMutableDictionaryRef = requireNotNull(CFBridgingRetain(dictionary)).reinterpret()
    return try {
        // Generic Kotlin/NSNumber bridging produces CFNumber, but Security requires CFBoolean.
        for (key in listOf(kSecReturnData, kSecReturnAttributes, kSecAttrSynchronizable)) {
            val value = dictionary.objectForKey(keychainString(key)) ?: continue
            val flag = (value as NSNumber).boolValue
            CFDictionarySetValue(retained, key, if (flag) kCFBooleanTrue else kCFBooleanFalse)
        }
        block(retained)
    } finally {
        CFRelease(retained)
    }
}

/** The returned Security object is retained; the caller must CFRelease it. */
internal fun copyKeychainResult(query: NSMutableDictionary): kotlinx.cinterop.COpaquePointer? = withCFDictionary(query) { cfQuery -> memScoped {
    val result = alloc<COpaquePointerVar>()
    val status = SecItemCopyMatching(cfQuery, result.ptr)
    when (status) {
        errSecSuccess -> result.value
        errSecItemNotFound -> null
        else -> throw SessionStoreException(SessionFailure.Unavailable)
    }
} }

private fun ByteArray.toNSData(): NSData = usePinned { pinned ->
    NSData.create(bytes = pinned.addressOf(0), length = size.toULong())
}
