package org.cuenora.shared.session

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/** Raw secrets: no data-class display, copying or automatic serialization. */
class SessionCredentials @Throws(IllegalArgumentException::class) constructor(val accessToken: String, val refreshToken: String) {
    init {
        require(validToken(accessToken) && validToken(refreshToken)) { "Invalid session credentials" }
    }
    override fun toString(): String = "SessionCredentials([REDACTED])"
}

private fun validToken(token: String) = token.length in 1..4096 && token.all { it in '!'..'~' }

enum class SessionFailure { Unavailable, Corrupt }

class SessionStoreException(val reason: SessionFailure) : Exception("Session storage: ${reason.name}")

/** One complete pair per backend namespace; absence is null, failures are never absence. */
interface SessionStore {
    @Throws(SessionStoreException::class)
    suspend fun read(): SessionCredentials?
    @Throws(SessionStoreException::class)
    suspend fun replace(credentials: SessionCredentials)
    @Throws(SessionStoreException::class)
    suspend fun clear()
}

internal const val MAX_SESSION_BYTES = 32768
private val sessionMutex = Mutex()

/** Serialize all store instances in this process, off the caller's UI thread. */
internal suspend fun <T> sessionOperation(block: () -> T): T = withContext(Dispatchers.IO) {
    sessionMutex.withLock {
        try { block() }
        catch (failure: SessionStoreException) { throw failure }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { throw SessionStoreException(SessionFailure.Unavailable) }
    }
}

internal fun sessionNamespace(value: String): String {
    require(value.isNotBlank() && value.length <= 1024 && value.none { it.isISOControl() }) { "Invalid session namespace" }
    return value.trimEnd('/')
}
