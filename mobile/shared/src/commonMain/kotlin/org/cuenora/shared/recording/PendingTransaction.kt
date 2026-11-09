package org.cuenora.shared.recording

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.TransactionForm
import org.cuenora.shared.validation.validateTransactionForm

/** A write-ahead record, not an offline draft. Save before POST; retain until acknowledged. */
class PendingTransaction(val owner: OwnerId, val command: TransactionCreateCommand, val key: IdempotencyKey) {
    init { require(command is TransactionCreateCommand.Income || command is TransactionCreateCommand.Expense || command is TransactionCreateCommand.Transfer) }

    override fun toString() = "PendingTransaction([REDACTED])"

    fun encode(): String = buildJsonObject {
        put("version", if (command is TransactionCreateCommand.Transfer) 2 else 1)
        put("owner_high", owner.highBits.toString()); put("owner_low", owner.lowBits.toString())
        put("key", key.value)
        val account: AccountId?
        val category: CategoryId?
        when (val value = command) {
            is TransactionCreateCommand.Income -> { put("kind", "income"); account = value.accountId; category = value.categoryId }
            is TransactionCreateCommand.Expense -> { put("kind", "expense"); account = value.accountId; category = value.categoryId }
            is TransactionCreateCommand.Transfer -> {
                put("kind", "transfer"); account = value.accountId; category = null
                put("destination_account_id", wireId(value.destinationAccountId))
                put("received_amount", writeMoney(value.receivedAmount))
                put("received_currency", value.receivedAmount.currency.name)
                put("quoted_rate", value.quotedRate?.let(::JsonPrimitive) ?: JsonNull)
            }
            else -> error("Unsupported pending transaction kind")
        }
        put("amount", writeMoney(command.amount))
        put("currency", command.amount.currency.name)
        put("date", wireDate(command.occurredOn))
        put("account_id", account?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
        put("category_id", category?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
        put("description", command.description?.let(::JsonPrimitive) ?: JsonNull)
    }.toString()

    companion object {
        fun decode(encoded: String): PendingTransaction = try {
            require(encoded.length <= 16384)
            val root = Json.parseToJsonElement(encoded).jsonObject
            val version = root.getValue("version").jsonPrimitive.int
            val common = setOf("version", "owner_high", "owner_low", "key", "kind", "amount", "currency", "date", "account_id", "category_id", "description")
            require(version in 1..2)
            require(root.keys == if (version == 1) common else common +
                setOf("destination_account_id", "received_amount", "received_currency", "quoted_rate"))
            val owner = OwnerId(root.string("owner_high").toLong(), root.string("owner_low").toLong())
            val key = IdempotencyKey(root.string("key"))
            val currency = Currency.valueOf(root.string("currency"))
            val amount = readMoney(root.getValue("amount"), currency)
            require(amount.minorUnits > 0)
            val date = businessDate(root.string("date"))
            val account = root.nullableString("account_id")?.let(::accountId)
            val category = root.nullableString("category_id")?.let(::categoryId)
            val description = root.nullableString("description")
            val kind = root.string("kind")
            require(if (version == 1) kind == "income" || kind == "expense" else kind == "transfer")
            val form = TransactionForm(
                kind = when (kind) { "income" -> TransactionKind.INCOME; "expense" -> TransactionKind.EXPENSE; else -> TransactionKind.TRANSFER },
                amount = (writeMoney(amount) as JsonPrimitive).content,
                currency = currency,
                occurredOn = wireDate(date),
                accountId = account,
                categoryId = category,
                description = description,
                destinationAccountId = if (version == 2) accountId(root.string("destination_account_id")) else null,
                receivedAmount = if (version == 2) root.string("received_amount") else null,
                receivedCurrency = if (version == 2) Currency.valueOf(root.string("received_currency")) else null,
                quotedRate = if (version == 2) root.nullableString("quoted_rate") else null,
            )
            require(validateTransactionForm(form).isLocallyValid)
            val command = form.toCommand()
            PendingTransaction(owner, command, key)
        } catch (failure: Exception) {
            if (failure is IllegalArgumentException && failure.message == "Invalid pending transaction") throw failure
            throw IllegalArgumentException("Invalid pending transaction", failure)
        }
    }
}

class PendingTransactionStoreException : Exception("Pending transaction storage unavailable")

interface PendingTransactionStore {
    suspend fun read(owner: OwnerId): PendingTransaction?
    suspend fun save(pending: PendingTransaction)
    suspend fun clear(owner: OwnerId)
}
