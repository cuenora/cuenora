package org.cuenora.shared.session

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlin.test.*

class SessionStoreTest {
    @Test
    fun versionedRecordPreservesBothSecretsWithoutDisplayLeaks() {
        val value = SessionCredentials("ca1_synthetic", "cr1_synthetic")
        val copy = SessionCodec.decode(SessionCodec.encode(value))
        assertEquals(value.accessToken, copy.accessToken)
        assertEquals(value.refreshToken, copy.refreshToken)
        assertEquals("SessionCredentials([REDACTED])", value.toString())
    }

    @Test
    fun partialMalformedUnknownAndOversizedRecordsAreCorrupt() {
        for (text in listOf("", "null", "{}", "{\"version\":2}",
            "{\"version\":1,\"access\":\"synthetic\"}",
            "{\"version\":1,\"access\":5,\"refresh\":\"synthetic\"}",
            "{\"version\":1,\"access\":\"\",\"refresh\":\"synthetic\"}")) {
            assertEquals(SessionFailure.Corrupt,
                assertFailsWith<SessionStoreException> { SessionCodec.decode(text.encodeToByteArray()) }.reason)
        }
        assertEquals(SessionFailure.Corrupt,
            assertFailsWith<SessionStoreException> { SessionCodec.decode(ByteArray(MAX_SESSION_BYTES + 1)) }.reason)
        assertEquals(SessionFailure.Corrupt,
            assertFailsWith<SessionStoreException> { SessionCodec.decode(byteArrayOf(-1)) }.reason)
    }

    @Test
    fun invalidCredentialsFailWithoutEchoingInput() {
        for (bad in listOf("", "with space", "line\nbreak", "é", "x".repeat(4097))) {
            assertEquals("Invalid session credentials",
                assertFailsWith<IllegalArgumentException> { SessionCredentials(bad, "synthetic") }.message)
        }
    }

    @Test
    fun storageFailuresAreRedactedAndCancellationPropagates() = runTest {
        val failure = assertFailsWith<SessionStoreException> {
            sessionOperation<Unit> { throw IllegalStateException("sensitive diagnostics") }
        }
        assertEquals(SessionFailure.Unavailable, failure.reason)
        assertNull(failure.cause)
        assertFalse(failure.toString().contains("sensitive diagnostics"))
        assertFailsWith<CancellationException> { sessionOperation<Unit> { throw CancellationException("cancel") } }
    }
}
