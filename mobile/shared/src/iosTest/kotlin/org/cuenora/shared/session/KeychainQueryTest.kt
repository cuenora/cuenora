@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class, kotlinx.cinterop.BetaInteropApi::class)

package org.cuenora.shared.session

import kotlinx.cinterop.reinterpret
import platform.CoreFoundation.CFBooleanGetTypeID
import platform.CoreFoundation.CFBooleanGetValue
import platform.CoreFoundation.CFDictionaryGetValue
import platform.CoreFoundation.CFGetTypeID
import platform.Security.kSecAttrSynchronizable
import platform.Security.kSecReturnAttributes
import platform.Security.kSecReturnData
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class KeychainQueryTest {
    @Test
    fun securityFlagsHaveNativeBooleanTypeAndValue() {
        for (expected in listOf(true, false)) {
            val dictionary = keychainDictionary(
                kSecReturnData to expected,
                kSecReturnAttributes to expected,
                kSecAttrSynchronizable to expected,
            )
            withCFDictionary(dictionary) { query ->
                for (key in listOf(kSecReturnData, kSecReturnAttributes, kSecAttrSynchronizable)) {
                    val value = assertNotNull(CFDictionaryGetValue(query, key))
                    assertEquals(CFBooleanGetTypeID(), CFGetTypeID(value))
                    assertEquals(expected, CFBooleanGetValue(value.reinterpret()))
                }
            }
        }
    }
}
