package org.cuenora.shared.history

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*

enum class TransactionVoidStage { Idle, Confirming, Busy, Uncertain, Complete, Blocked }
data class TransactionVoidState(
    val stage: TransactionVoidStage = TransactionVoidStage.Idle,
    val transaction: Transaction? = null,
    val targetId: TransactionId? = null,
    val recovered: Boolean = false,
    val message: String? = null,
)

/** Immutable owner-bound write-ahead record. The same operation and key are retained until acknowledged. */
class PendingTransactionVoid(val owner: OwnerId, val transactionId: TransactionId, val key: IdempotencyKey) {
    override fun toString() = "PendingTransactionVoid([REDACTED])"
    internal fun encode() = "${owner.highBits}|${owner.lowBits}|${transactionId.highBits}|${transactionId.lowBits}|${key.value}"
    override fun equals(other: Any?) = other is PendingTransactionVoid && encode() == other.encode()
    override fun hashCode() = encode().hashCode()
}

interface PendingTransactionVoidStore {
    suspend fun read(owner: OwnerId): PendingTransactionVoid?
    suspend fun save(pending: PendingTransactionVoid)
    suspend fun clear(owner: OwnerId)
}

class TransactionVoidStoreException : Exception("Pending transaction void storage unavailable")

interface TransactionVoidBackend {
    fun clear() {}
    suspend fun profile(): RepositoryResult<OnboardingProfile>
    suspend fun void(owner: OwnerId, transactionId: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement>
}

class AuthenticatedTransactionVoidBackend(private val session: AuthSession, private val api: ApiClient) : TransactionVoidBackend {
    private var observedOwner: OwnerId? = null
    private var retired = false
    override fun clear() { retired = true }

    override suspend fun profile(): RepositoryResult<OnboardingProfile> = session.authorized { token ->
        if (retired) return@authorized denied()
        when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> if (retired || !remember(result.value.ownerId) || !result.value.emailVerified) denied()
                else RepositoryResult.Success(result.value)
        }
    }

    override suspend fun void(owner: OwnerId, transactionId: TransactionId, key: IdempotencyKey) = session.authorized { token ->
        if (retired) return@authorized denied()
        when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> if (retired || !remember(result.value.ownerId) || result.value.ownerId != owner || !result.value.emailVerified)
                denied()
            else HttpTransactionRepository(api, RepositoryContext(owner, token)).void(transactionId, key)
        }
    }

    private fun remember(owner: OwnerId): Boolean {
        val previous = observedOwner
        if (previous != null && previous != owner) return false
        observedOwner = owner
        return true
    }

    private fun denied() = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
}

/** Owns one authenticated UI lifetime; retirement prevents any suspended work publishing into its replacement. */
class TransactionVoidSession(
    private val backend: TransactionVoidBackend,
    private val store: PendingTransactionVoidStore,
    private val newKey: () -> IdempotencyKey,
) {
    private val mutable = MutableStateFlow(TransactionVoidState())
    val state = mutable.asStateFlow()
    private val mutex = Mutex()
    private var retired = false
    private var retryingPriorOperation = false

    fun request(transaction: Transaction) {
        if (retired || mutable.value.stage !in setOf(TransactionVoidStage.Idle, TransactionVoidStage.Complete) || transaction.state != TransactionState.POSTED) return
        retryingPriorOperation = false
        mutable.value = TransactionVoidState(TransactionVoidStage.Confirming, transaction, transaction.id)
    }

    /** Recover a prior write-ahead operation without depending on the target appearing in history. */
    suspend fun restorePending(): Boolean {
        if (retired || mutable.value.stage != TransactionVoidStage.Idle || !mutex.tryLock()) return false
        mutable.value = TransactionVoidState(stage = TransactionVoidStage.Busy)
        return try {
            val profile = when (val result = backend.profile()) {
                is RepositoryResult.Failure -> { if (!retired) mutable.value = TransactionVoidState(); return false }
                is RepositoryResult.Success -> result.value
            }
            if (retired) return false
            if (!profile.emailVerified) { mutable.value = TransactionVoidState(); return false }
            val pending = store.read(profile.ownerId)
            if (pending == null) { if (!retired) mutable.value = TransactionVoidState(); return false }
            if (retired) return false
            retryingPriorOperation = true
            mutable.value = TransactionVoidState(
                stage = TransactionVoidStage.Uncertain,
                targetId = pending.transactionId,
                recovered = true,
                message = "A previous void has no confirmed result. Retry only that original operation.",
            )
            true
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            if (!retired) mutable.value = TransactionVoidState(
                stage = TransactionVoidStage.Blocked,
                message = "The pending transaction operation could not be read from secure storage.",
            )
            false
        } finally { mutex.unlock() }
    }

    fun retryOriginal() {
        if (!retired && mutable.value.stage == TransactionVoidStage.Uncertain) {
            retryingPriorOperation = true
            mutable.value = mutable.value.copy(stage = TransactionVoidStage.Confirming, message = null)
        }
    }

    fun cancel() {
        if (!retired && mutable.value.stage == TransactionVoidStage.Confirming) {
            mutable.value = if (retryingPriorOperation) mutable.value.copy(stage = TransactionVoidStage.Uncertain) else TransactionVoidState()
        }
    }

    /** Close presentation only; the encrypted original remains available for recovery. */
    fun deferPending() {
        if (!retired && mutable.value.stage == TransactionVoidStage.Uncertain) mutable.value = TransactionVoidState()
    }

    fun dismiss() {
        if (!retired && mutable.value.stage in setOf(TransactionVoidStage.Complete, TransactionVoidStage.Blocked)) {
            retryingPriorOperation = false
            mutable.value = TransactionVoidState()
        }
    }

    fun clear() { retired = true; backend.clear(); retryingPriorOperation = false; mutable.value = TransactionVoidState() }

    suspend fun confirm(): Boolean {
        val selected = mutable.value.transaction
        val targetId = mutable.value.targetId ?: return false
        if (retired || mutable.value.stage != TransactionVoidStage.Confirming || !mutex.tryLock()) return false
        return try {
            mutable.value = mutable.value.copy(stage = TransactionVoidStage.Busy, message = null)
            val profile = when (val result = backend.profile()) {
                is RepositoryResult.Failure -> return if (retryingPriorOperation) uncertain("Sign in again or check your connection, then retry the original void.")
                    else blocked("Sign-in or account verification is unavailable. No new operation was submitted.")
                is RepositoryResult.Success -> result.value
            }
            if (retired) return false
            if (!profile.emailVerified) return if (retryingPriorOperation) uncertain("Verify your email, then retry the original void.")
                else blocked("Verify your email before changing financial records.")
            var pending = store.read(profile.ownerId)
            if (retired) return false
            if (pending != null) retryingPriorOperation = true
            if (pending == null) {
                if (selected == null) return blocked("The original transaction is unavailable. No replacement operation was submitted.")
                pending = PendingTransactionVoid(profile.ownerId, targetId, newKey())
                store.save(pending)
                if (retired) return false
            } else if (pending.transactionId != targetId) {
                mutable.value = TransactionVoidState(TransactionVoidStage.Uncertain, targetId = pending.transactionId, recovered = true,
                    message = "A previous void needs confirmation. Retry only that original operation.")
                return false
            }
            val result = backend.void(pending.owner, pending.transactionId, pending.key)
            if (retired) return false
            when (result) {
                is RepositoryResult.Failure -> {
                    if (!retryingPriorOperation && result.error.isDefinitiveVoidRejection()) {
                        store.clear(pending.owner)
                        blocked("The server rejected this void. The transaction was not changed.")
                    } else {
                        retryingPriorOperation = true
                        uncertain("The result is not confirmed. Retry this same transaction; do not submit a new operation.")
                    }
                }
                is RepositoryResult.Success -> {
                    if (result.value.id != pending.transactionId || result.value.state != TransactionState.VOIDED)
                        return uncertain("The server response did not confirm this transaction void. Retry the original operation.")
                    store.clear(pending.owner)
                    if (retired) return false
                    retryingPriorOperation = false
                    mutable.value = TransactionVoidState(
                        stage = TransactionVoidStage.Complete,
                        transaction = selected,
                        targetId = pending.transactionId,
                        recovered = selected == null,
                        message = "Transaction voided. History is retained.",
                    )
                    true
                }
            }
        } catch (cancelled: CancellationException) {
            if (!retired) {
                retryingPriorOperation = true
                uncertain("The result is not confirmed. Retry this same transaction; do not submit a new operation.")
            }
            throw cancelled
        } catch (_: Exception) {
            if (retired) false else {
                retryingPriorOperation = true
                uncertain("Secure storage or completion is unavailable. Retry only this original operation.")
            }
        } finally { mutex.unlock() }
    }

    private fun blocked(message: String): Boolean {
        if (!retired) mutable.value = mutable.value.copy(stage = TransactionVoidStage.Blocked, message = message)
        return false
    }

    private fun uncertain(message: String): Boolean {
        if (!retired) mutable.value = mutable.value.copy(stage = TransactionVoidStage.Uncertain, message = message)
        return false
    }
}

private fun ApiFailure.isDefinitiveVoidRejection(): Boolean =
    (this as? ApiFailure.Http)?.status?.let { it in 400..499 && it !in setOf(408, 425, 429) } == true

fun authenticatedTransactionVoidSession(
    session: AuthSession,
    api: ApiClient,
    store: PendingTransactionVoidStore,
    newKey: () -> IdempotencyKey,
) = TransactionVoidSession(AuthenticatedTransactionVoidBackend(session, api), store, newKey)
