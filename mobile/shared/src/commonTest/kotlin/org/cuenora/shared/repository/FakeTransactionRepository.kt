package org.cuenora.shared.repository

import org.cuenora.shared.domain.TransactionId

sealed interface TransactionRepositoryCall {
    data class List(val query: TransactionQuery) : TransactionRepositoryCall
    data class Create(val command: TransactionCreateCommand, val key: IdempotencyKey) : TransactionRepositoryCall
    data class Edit(val id: TransactionId, val patch: TransactionPatch, val key: IdempotencyKey) : TransactionRepositoryCall
    data class Void(val id: TransactionId, val key: IdempotencyKey) : TransactionRepositoryCall
}

/** Scriptable boundary fake; it captures calls but does not emulate server financial rules. */
class FakeTransactionRepository : TransactionRepository {
    val calls = mutableListOf<TransactionRepositoryCall>()
    var listResult: RepositoryResult<Page<TransactionRecord>> =
        RepositoryResult.Success(Page(emptyList(), null))
    var createResult: RepositoryResult<TransactionRecord> = RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.InvalidResponse(0, null))
    var editResult: RepositoryResult<TransactionAcknowledgement> = RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.InvalidResponse(0, null))
    var voidResult: RepositoryResult<TransactionAcknowledgement> = RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.InvalidResponse(0, null))
    var beforeCall: suspend (TransactionRepositoryCall) -> Unit = {}

    override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
        val call = TransactionRepositoryCall.List(query)
        beforeCall(call)
        calls += call
        return listResult
    }

    override suspend fun create(command: TransactionCreateCommand, key: IdempotencyKey): RepositoryResult<TransactionRecord> {
        val call = TransactionRepositoryCall.Create(command, key)
        beforeCall(call)
        calls += call
        return createResult
    }

    override suspend fun edit(id: TransactionId, patch: TransactionPatch, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> {
        val call = TransactionRepositoryCall.Edit(id, patch, key)
        beforeCall(call)
        calls += call
        return editResult
    }

    override suspend fun void(id: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> {
        val call = TransactionRepositoryCall.Void(id, key)
        beforeCall(call)
        calls += call
        return voidResult
    }
}
