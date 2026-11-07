package org.cuenora.shared.repository

import org.cuenora.shared.domain.Account

/** A deterministic scripted fake. It captures calls but does not model server rules. */
class FakeAccountRepository(
    val listResults: MutableList<RepositoryResult<Page<AccountWithBalance>>> = mutableListOf(),
    val createResults: MutableList<RepositoryResult<Account>> = mutableListOf(),
    val updateResults: MutableList<RepositoryResult<Account>> = mutableListOf(),
) : AccountRepository {
    val calls = mutableListOf<Call>()

    override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
        calls += Call.List(query)
        return listResults.removeFirstOrNull() ?: error("No list result was scripted")
    }

    override suspend fun create(command: CreateAccountCommand, idempotencyKey: IdempotencyKey): RepositoryResult<Account> {
        calls += Call.Create(command, idempotencyKey)
        return createResults.removeFirstOrNull() ?: error("No create result was scripted")
    }

    override suspend fun update(command: UpdateAccountCommand): RepositoryResult<Account> {
        calls += Call.Update(command)
        return updateResults.removeFirstOrNull() ?: error("No update result was scripted")
    }

    sealed interface Call {
        data class List(val query: AccountListQuery) : Call
        data class Create(val command: CreateAccountCommand, val idempotencyKey: IdempotencyKey) : Call
        data class Update(val command: UpdateAccountCommand) : Call
    }
}
