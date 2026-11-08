package org.cuenora.shared.auth

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import org.cuenora.shared.validation.*
import kotlin.time.TimeSource

// Public presentation snapshots intentionally contain no credentials or owner identifiers.
enum class AuthPhase { Restoring, SignedOut, Working, SignedIn, Renewing, StorageBlocked }
enum class AuthNotice {
    RegistrationAccepted, InvalidCredentials, RateLimited, NetworkUnavailable,
    ServiceUnavailable, SessionExpired, SessionUncertain, StorageUnavailable, RemoteLogoutUnconfirmed,
}
data class AuthState(
    val phase: AuthPhase = AuthPhase.Restoring,
    val notice: AuthNotice? = null,
    val issues: List<ValidationIssue> = emptyList(),
)

/**
 * One process-owned instance per backend/store. Serializes the whole network/storage
 * transaction, not just individual store operations. Never share its store with another
 * coordinator. Restoration rotates because persisted credentials carry no trusted expiry.
 */
class AuthSession(
    private val repository: AuthRepository,
    private val store: SessionStore,
    private val nowMillis: () -> Long = monotonicClock(),
) {
    private val mutex = Mutex()
    private val mutableState = MutableStateFlow(AuthState())
    val state: StateFlow<AuthState> = mutableState.asStateFlow()
    private var initialized = false
    private var credentials: SessionCredentials? = null
    private var renewAt = 0L

    suspend fun restore() = mutex.withLock {
        if (initialized) return@withLock
        initialized = true
        guarded {
            credentials = store.read()
            if (credentials == null) publish(AuthPhase.SignedOut)
            else rotate()
        }
    }

    suspend fun register(email: String, password: String) = mutex.withLock {
        if (credentials != null || state.value.phase == AuthPhase.StorageBlocked) return@withLock
        val validation = validateRegistrationForm(email, password)
        if (!validation.isLocallyValid) {
            mutableState.value = AuthState(AuthPhase.SignedOut, issues = validation.issues)
            return@withLock
        }
        initialized = true
        guarded {
            publish(AuthPhase.Working)
            when (val result = repository.register(email, password)) {
                is RepositoryResult.Success -> publish(AuthPhase.SignedOut, AuthNotice.RegistrationAccepted)
                is RepositoryResult.Failure -> publish(AuthPhase.SignedOut, notice(result.error))
            }
        }
    }

    suspend fun login(email: String, password: String) = mutex.withLock {
        if (credentials != null || state.value.phase == AuthPhase.StorageBlocked) return@withLock
        val validation = validateLoginForm(email, password)
        if (!validation.isLocallyValid) {
            mutableState.value = AuthState(AuthPhase.SignedOut, issues = validation.issues)
            return@withLock
        }
        initialized = true
        guarded {
            publish(AuthPhase.Working)
            val started = nowMillis()
            when (val result = repository.login(email, password)) {
                is RepositoryResult.Success -> accept(result.value, started)
                is RepositoryResult.Failure -> publish(AuthPhase.SignedOut, notice(result.error))
            }
        }
    }

    /** Due checks coalesce concurrent callers; never blindly replay a refresh generation. */
    suspend fun renewIfNeeded() = mutex.withLock {
        if (state.value.phase != AuthPhase.SignedIn || nowMillis() < renewAt) return@withLock
        guarded { rotate() }
    }

    suspend fun logout() = mutex.withLock {
        initialized = true
        val previous = credentials
        val unknownRemoteSession = state.value.phase == AuthPhase.StorageBlocked && previous == null
        guarded {
            publish(AuthPhase.Working)
            // Local removal happens first, so a crash/offline response cannot restore this pair.
            store.clear()
            credentials = null
            publish(AuthPhase.SignedOut, AuthNotice.RemoteLogoutUnconfirmed)
            val confirmed = if (previous == null) !unknownRemoteSession else {
                repository.logout(previous) is RepositoryResult.Success
            }
            publish(AuthPhase.SignedOut, if (confirmed) null else AuthNotice.RemoteLogoutUnconfirmed)
        }
    }

    private suspend fun rotate() {
        val previous = credentials ?: return
        publish(if (state.value.phase == AuthPhase.SignedIn) AuthPhase.Renewing else AuthPhase.Working)
        // Destructive consumption is intentional: a process crash or lost HTTP response
        // must not leave an old generation available for replay (which revokes the family).
        // Tradeoff: interrupted/offline renewal requires a new sign-in, not an unsafe retry.
        store.clear()
        credentials = null
        val started = nowMillis()
        when (val result = repository.refresh(previous)) {
            is RepositoryResult.Success -> accept(result.value, started)
            is RepositoryResult.Failure -> publish(AuthPhase.SignedOut,
                if ((result.error as? ApiFailure.Http)?.status == 401) AuthNotice.SessionExpired
                else AuthNotice.SessionUncertain)
        }
    }

    private suspend fun accept(grant: TokenGrant, started: Long) {
        credentials = grant.credentials
        // No signed-in snapshot is published before durable pair replacement succeeds.
        store.replace(grant.credentials)
        renewAt = started + (grant.expiresInSeconds - 30).coerceAtLeast(1) * 1000
        publish(AuthPhase.SignedIn)
    }

    private suspend fun guarded(block: suspend () -> Unit) {
        try { block() }
        catch (cancelled: CancellationException) {
            // A store call may commit then observe cancellation. Remove any ambiguous pair
            // before propagating; cleanup cannot itself be cancelled by the abandoned UI.
            withContext(NonCancellable) {
                try {
                    store.clear()
                    credentials = null
                    val message = if (state.value.notice == AuthNotice.RemoteLogoutUnconfirmed)
                        AuthNotice.RemoteLogoutUnconfirmed else AuthNotice.SessionUncertain
                    publish(AuthPhase.SignedOut, message)
                } catch (_: SessionStoreException) {
                    publish(AuthPhase.StorageBlocked, AuthNotice.StorageUnavailable)
                }
            }
            throw cancelled
        } catch (_: SessionStoreException) {
            publish(AuthPhase.StorageBlocked, AuthNotice.StorageUnavailable)
        }
    }

    private fun publish(phase: AuthPhase, notice: AuthNotice? = null) {
        mutableState.value = AuthState(phase, notice)
    }

    private fun notice(error: ApiFailure): AuthNotice = when (error) {
        ApiFailure.Network, ApiFailure.Timeout -> AuthNotice.NetworkUnavailable
        is ApiFailure.Http -> when (error.status) {
            401 -> AuthNotice.InvalidCredentials
            429 -> AuthNotice.RateLimited
            else -> AuthNotice.ServiceUnavailable
        }
        is ApiFailure.InvalidResponse -> AuthNotice.ServiceUnavailable
    }
}

private fun monotonicClock(): () -> Long {
    val origin = TimeSource.Monotonic.markNow()
    return { origin.elapsedNow().inWholeMilliseconds }
}
