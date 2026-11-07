package org.cuenora.shared.repository

import io.ktor.client.request.*
import io.ktor.http.*
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient

/** Account metadata and its exact current balance, which is not part of [Account]. */
data class AccountWithBalance(val account: Account, val balance: Money) {
    init { require(balance.currency == account.currency) { "Balance currency must match the account" } }
}

data class AccountListQuery(val limit: Int? = null, val cursor: String? = null) {
    init { require(limit == null || limit in 1..100) { "Account page limit must be from 1 through 100" } }
}

data class OpeningBalance(val amount: Money, val occurredOn: BusinessDate)

data class CreateAccountCommand(
    val name: AccountName,
    val type: AccountType,
    val currency: Currency,
    val openingBalance: OpeningBalance? = null,
) {
    init { require(openingBalance == null || openingBalance.amount.currency == currency) {
        "Opening balance currency must match the account"
    } }
}

data class UpdateAccountCommand(
    val id: AccountId,
    val name: PatchField<AccountName> = PatchField.Absent,
    val status: PatchField<AccountStatus> = PatchField.Absent,
) {
    init {
        require(name !is PatchField.Null && status !is PatchField.Null) { "Account fields cannot be cleared" }
        require(name !is PatchField.Absent || status !is PatchField.Absent) { "Account update must change a field" }
    }
}

interface AccountRepository {
    suspend fun list(query: AccountListQuery = AccountListQuery()): RepositoryResult<Page<AccountWithBalance>>
    suspend fun create(command: CreateAccountCommand, idempotencyKey: IdempotencyKey): RepositoryResult<Account>
    suspend fun update(command: UpdateAccountCommand): RepositoryResult<Account>
}

/** HTTP adapter for the existing account resource endpoints; it never owns [api]'s lifecycle. */
class HttpAccountRepository(
    private val api: ApiClient,
    private val context: RepositoryContext,
) : AccountRepository {
    override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
        val path = queryPath("/v1/accounts", listOf("limit" to query.limit?.toString(), "cursor" to query.cursor))
        return mapResponse(api.request(path, headers = context.headers()), 200) { element ->
            val root = element.jsonObject
            require(root.keys.containsAll(setOf("accounts", "next_cursor"))) { "Invalid account page" }
            val items = root.getValue("accounts").jsonArray.map { decodeAccount(it, withBalance = true).let { decoded ->
                AccountWithBalance(decoded.account, decoded.balance!!)
            } }
            Page(items, root.nullableString("next_cursor"))
        }
    }

    override suspend fun create(
        command: CreateAccountCommand,
        idempotencyKey: IdempotencyKey,
    ): RepositoryResult<Account> {
        val body = buildJsonObject {
            put("name", command.name.value)
            put("account_type", command.type.toWire())
            put("currency", command.currency.name)
            command.openingBalance?.let { opening ->
                put("opening_balance", buildJsonObject {
                    put("amount", writeMoney(opening.amount))
                    put("occurred_on", wireDate(opening.occurredOn))
                })
            }
        }
        return mapResponse(api.request("/v1/accounts", HttpMethod.Post, body,
            context.headers(idempotencyKey)), 201) {
            decodeAccount(it, withBalance = false).account.also { account ->
                require(account.name == command.name && account.type == command.type &&
                    account.currency == command.currency && account.status == AccountStatus.ACTIVE) {
                    "Account creation response does not match request"
                }
            }
        }
    }

    override suspend fun update(command: UpdateAccountCommand): RepositoryResult<Account> {
        val body = buildJsonObject {
            patch("name", command.name) { JsonPrimitive(it.value) }
            patch("status", command.status) { JsonPrimitive(it.toWire()) }
        }
        return mapResponse(api.request("/v1/accounts/${wireId(command.id)}", HttpMethod.Patch,
            body, context.headers()), 200) {
            decodeAccount(it, withBalance = false).account.also { account ->
                require(account.id == command.id) { "Account update response identity does not match request" }
                if (command.name is PatchField.Value) require(account.name == command.name.value) {
                    "Account update response name does not match request"
                }
                if (command.status is PatchField.Value) require(account.status == command.status.value) {
                    "Account update response status does not match request"
                }
            }
        }
    }

    private fun decodeAccount(element: JsonElement, withBalance: Boolean): DecodedAccount {
        val value = element.jsonObject
        val fields = if (withBalance) setOf("id", "name", "account_type", "currency", "status", "balance")
            else setOf("id", "name", "account_type", "currency", "status")
        require(value.keys.containsAll(fields)) { "Invalid account representation" }
        val currency = Currency.valueOf(value.string("currency"))
        val account = Account(
            id = accountId(value.string("id")),
            ownerId = context.ownerId,
            name = AccountName(value.string("name")),
            type = when (value.string("account_type")) {
                "normal" -> AccountType.NORMAL
                "savings" -> AccountType.SAVINGS
                else -> throw IllegalArgumentException("Invalid account type")
            },
            currency = currency,
            status = when (value.string("status")) {
                "active" -> AccountStatus.ACTIVE
                "archived" -> AccountStatus.ARCHIVED
                else -> throw IllegalArgumentException("Invalid account status")
            },
        )
        return DecodedAccount(account, if (withBalance) readMoney(value.getValue("balance"), currency) else null)
    }

    private data class DecodedAccount(val account: Account, val balance: Money?)
}

private fun AccountType.toWire() = when (this) {
    AccountType.NORMAL -> "normal"
    AccountType.SAVINGS -> "savings"
}

private fun AccountStatus.toWire() = when (this) {
    AccountStatus.ACTIVE -> "active"
    AccountStatus.ARCHIVED -> "archived"
}
