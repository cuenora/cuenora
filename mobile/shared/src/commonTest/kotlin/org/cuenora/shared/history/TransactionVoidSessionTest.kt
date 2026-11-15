package org.cuenora.shared.history

import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TransactionVoidSessionTest {
    private val owner = OwnerId(11, 12)
    private val transaction = Transaction(TransactionId(21, 22), TransactionKind.EXPENSE, TransactionState.POSTED,
        Money(1250, Currency.SEK), BusinessDate(2026, 1, 2))

    @Test fun cancelBeforeConfirmationDoesNotWriteOrSubmit() = runTest {
        val store = Store(); val backend = Backend()
        val session = session(backend, store)
        session.request(transaction)
        session.cancel()
        assertFalse(session.confirm())
        assertEquals(0, store.saves)
        assertEquals(0, backend.voids.size)
    }

    @Test fun uncertainResultRetainsOriginalKeyAndOnlyRetriesThatTransaction() = runTest {
        val store = Store(); val backend = Backend().apply {
            responses += RepositoryResult.Failure(ApiFailure.Network)
            responses += RepositoryResult.Success(TransactionAcknowledgement(transaction.id, TransactionState.VOIDED))
        }
        val session = session(backend, store)
        session.request(transaction)
        assertFalse(session.confirm())
        assertEquals(TransactionVoidStage.Uncertain, session.state.value.stage)
        val first = store.pending
        session.retryOriginal()
        assertTrue(session.confirm())
        assertEquals(2, backend.voids.size)
        assertEquals(backend.voids[0], backend.voids[1])
        assertEquals(first, backend.voids[0])
        assertNull(store.pending)
        assertEquals(TransactionVoidStage.Complete, session.state.value.stage)
    }

    @Test fun unresolvedDifferentOperationCannotBeReplaced() = runTest {
        val existing = PendingTransactionVoid(owner, TransactionId(30, 31), IdempotencyKey("prior-key-123"))
        val store = Store().apply { pending = existing }
        val backend = Backend()
        val session = session(backend, store)
        session.request(transaction)
        assertFalse(session.confirm())
        assertEquals(0, store.saves)
        assertEquals(existing, store.pending)
        assertEquals(0, backend.voids.size)
        assertEquals(TransactionVoidStage.Uncertain, session.state.value.stage)
        assertEquals(existing.transactionId, session.state.value.targetId)
    }

    @Test fun retirementIgnoresLateResult() = runTest {
        val store = Store(); val backend = Backend()
        val session = session(backend, store)
        session.request(transaction)
        session.clear()
        assertFalse(session.confirm())
        assertEquals(TransactionVoidStage.Idle, session.state.value.stage)
        assertEquals(0, store.saves)
    }

    @Test fun restartRecoversWithoutAVisibleTransactionAndKeepsOriginalKey() = runTest {
        val original = PendingTransactionVoid(owner, transaction.id, IdempotencyKey("restart-key-123"))
        val store = Store().apply { pending = original }
        val backend = Backend().apply {
            responses += RepositoryResult.Success(TransactionAcknowledgement(transaction.id, TransactionState.VOIDED))
        }
        val session = session(backend, store)
        assertTrue(session.restorePending())
        assertNull(session.state.value.transaction)
        session.retryOriginal()
        assertTrue(session.confirm())
        assertEquals(listOf(original), backend.voids)
        assertEquals(0, store.saves)
        assertNull(store.pending)
    }

    @Test fun cancellingRecoveredConfirmationDoesNotLoseOriginalRecovery() = runTest {
        val store = Store().apply { pending = PendingTransactionVoid(owner, transaction.id, IdempotencyKey("prior-key-123")) }
        val session = session(Backend(), store)
        session.restorePending()
        session.retryOriginal()
        session.cancel()
        assertEquals(TransactionVoidStage.Uncertain, session.state.value.stage)
        assertEquals(transaction.id, session.state.value.targetId)
    }

    @Test fun preexistingJournalIsNotClearedByALaterDefinitiveRejection() = runTest {
        val original = PendingTransactionVoid(owner, transaction.id, IdempotencyKey("prior-key-123"))
        val store = Store().apply { pending = original }
        val backend = Backend().apply { responses += RepositoryResult.Failure(ApiFailure.Http(403, null, null, null)) }
        val session = session(backend, store)
        session.request(transaction)
        assertFalse(session.confirm())
        assertEquals(original, store.pending)
        assertEquals(TransactionVoidStage.Uncertain, session.state.value.stage)
    }

    @Test fun firstDefinitiveRejectionReleasesNewJournalWithoutClaimingSuccess() = runTest {
        val store = Store()
        val backend = Backend().apply { responses += RepositoryResult.Failure(ApiFailure.Http(409, null, null, null)) }
        val session = session(backend, store)
        session.request(transaction)
        assertFalse(session.confirm())
        assertNull(store.pending)
        assertEquals(TransactionVoidStage.Blocked, session.state.value.stage)
    }

    private fun session(backend: Backend, store: Store) = TransactionVoidSession(backend, store) { IdempotencyKey("new-key-123") }

    private class Store : PendingTransactionVoidStore {
        var pending: PendingTransactionVoid? = null
        var saves = 0
        override suspend fun read(owner: OwnerId) = pending?.also { assertEquals(owner, it.owner) }
        override suspend fun save(pending: PendingTransactionVoid) { saves++; this.pending = pending }
        override suspend fun clear(owner: OwnerId) { assertEquals(owner, pending?.owner); pending = null }
    }

    private inner class Backend : TransactionVoidBackend {
        val voids = mutableListOf<PendingTransactionVoid>()
        val responses = ArrayDeque<RepositoryResult<TransactionAcknowledgement>>()
        override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
        override suspend fun void(owner: OwnerId, transactionId: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> {
            val pending = PendingTransactionVoid(owner, transactionId, key)
            voids += pending
            return responses.removeFirstOrNull() ?: RepositoryResult.Failure(ApiFailure.Network)
        }
    }
}
