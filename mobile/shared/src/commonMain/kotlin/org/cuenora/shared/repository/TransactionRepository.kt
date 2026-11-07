package org.cuenora.shared.repository

import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*

sealed interface TransactionAccountFilter {
    data class Linked(val accountId: AccountId) : TransactionAccountFilter
    data object Accountless : TransactionAccountFilter
}

data class TransactionQuery(
    val limit: Int = 50,
    val cursor: String? = null,
    val dateFrom: BusinessDate? = null,
    val dateTo: BusinessDate? = null,
    val categoryId: CategoryId? = null,
    val account: TransactionAccountFilter? = null,
) {
    init { require(limit in 1..100) { "Transaction page limit must be between 1 and 100" } }
}

sealed interface TransactionCreateCommand {
    val amount: Money
    val occurredOn: BusinessDate
    val description: String?

    data class Income(
        override val amount: Money,
        override val occurredOn: BusinessDate,
        val accountId: AccountId? = null,
        val categoryId: CategoryId? = null,
        override val description: String? = null,
    ) : TransactionCreateCommand

    data class Expense(
        override val amount: Money,
        override val occurredOn: BusinessDate,
        val accountId: AccountId? = null,
        val categoryId: CategoryId? = null,
        override val description: String? = null,
    ) : TransactionCreateCommand

    data class InterestIncome(
        override val amount: Money,
        override val occurredOn: BusinessDate,
        val accountId: AccountId,
        val categoryId: CategoryId? = null,
        override val description: String? = null,
    ) : TransactionCreateCommand

    data class Transfer(
        override val amount: Money,
        val accountId: AccountId,
        val receivedAmount: Money,
        val destinationAccountId: AccountId,
        override val occurredOn: BusinessDate,
        val quotedRate: String? = null,
        override val description: String? = null,
    ) : TransactionCreateCommand
}

data class TransactionPatch(
    val amount: PatchField<DecimalAmount> = PatchField.Absent,
    val currency: PatchField<Currency> = PatchField.Absent,
    val occurredOn: PatchField<BusinessDate> = PatchField.Absent,
    val accountId: PatchField<AccountId> = PatchField.Absent,
    val destinationAccountId: PatchField<AccountId> = PatchField.Absent,
    val receivedAmount: PatchField<DecimalAmount> = PatchField.Absent,
    val receivedCurrency: PatchField<Currency> = PatchField.Absent,
    val categoryId: PatchField<CategoryId> = PatchField.Absent,
    val description: PatchField<String> = PatchField.Absent,
) {
    init {
        require(listOf(amount, currency, occurredOn, accountId, destinationAccountId, receivedAmount,
            receivedCurrency, categoryId, description).any { it !is PatchField.Absent }) {
            "Transaction patch must change at least one field"
        }
        require(amount !is PatchField.Null && currency !is PatchField.Null && occurredOn !is PatchField.Null) {
            "Non-nullable transaction fields cannot be cleared"
        }
    }
}

data class RateComparison(
    val indicativeReceivedAmount: Money,
    val difference: Money,
    val mismatch: Boolean,
)

data class TransactionRecord(
    val transaction: Transaction,
    val quotedRate: String? = null,
    val rateComparison: RateComparison? = null,
)

data class TransactionAcknowledgement(val id: TransactionId, val state: TransactionState)

interface TransactionRepository {
    suspend fun list(query: TransactionQuery = TransactionQuery()): RepositoryResult<Page<TransactionRecord>>
    suspend fun create(command: TransactionCreateCommand, key: IdempotencyKey): RepositoryResult<TransactionRecord>
    suspend fun edit(id: TransactionId, patch: TransactionPatch, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement>
    suspend fun void(id: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement>
}

/** HTTP adapter only: transport and session/key lifetimes remain caller-owned. */
class HttpTransactionRepository(
    private val api: ApiClient,
    private val context: RepositoryContext,
) : TransactionRepository {
    override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
        val account = when (val filter = query.account) {
            null -> null
            TransactionAccountFilter.Accountless -> "none"
            is TransactionAccountFilter.Linked -> wireId(filter.accountId)
        }
        val path = queryPath("/v1/transactions", listOf(
            "limit" to query.limit.toString(),
            "cursor" to query.cursor,
            "date_from" to query.dateFrom?.let(::wireDate),
            "date_to" to query.dateTo?.let(::wireDate),
            "category_id" to query.categoryId?.let(::wireId),
            "account_id" to account,
        ))
        return mapResponse(api.request(path, headers = context.headers()), 200, ::readListing)
    }

    override suspend fun create(command: TransactionCreateCommand, key: IdempotencyKey): RepositoryResult<TransactionRecord> {
        val body = buildJsonObject {
            when (command) {
                is TransactionCreateCommand.Income -> {
                    put("kind", "income")
                    put("amount", writeMoney(command.amount))
                    put("currency", command.amount.currency.name)
                    put("occurred_on", wireDate(command.occurredOn))
                    putNullableId("account_id", command.accountId?.let(::wireId))
                    putNullableId("category_id", command.categoryId?.let(::wireId))
                    putNullableString("description", command.description)
                }
                is TransactionCreateCommand.Expense -> {
                    put("kind", "expense")
                    put("amount", writeMoney(command.amount))
                    put("currency", command.amount.currency.name)
                    put("occurred_on", wireDate(command.occurredOn))
                    putNullableId("account_id", command.accountId?.let(::wireId))
                    putNullableId("category_id", command.categoryId?.let(::wireId))
                    putNullableString("description", command.description)
                }
                is TransactionCreateCommand.InterestIncome -> {
                    put("kind", "interest_income")
                    put("amount", writeMoney(command.amount))
                    put("currency", command.amount.currency.name)
                    put("occurred_on", wireDate(command.occurredOn))
                    put("account_id", wireId(command.accountId))
                    putNullableId("category_id", command.categoryId?.let(::wireId))
                    putNullableString("description", command.description)
                }
                is TransactionCreateCommand.Transfer -> {
                    put("kind", "transfer")
                    put("amount", writeMoney(command.amount))
                    put("currency", command.amount.currency.name)
                    put("account_id", wireId(command.accountId))
                    put("destination_account_id", wireId(command.destinationAccountId))
                    put("received_amount", writeMoney(command.receivedAmount))
                    put("received_currency", command.receivedAmount.currency.name)
                    put("occurred_on", wireDate(command.occurredOn))
                    putNullableString("description", command.description)
                    command.quotedRate?.let { put("quoted_rate", it) }
                }
            }
        }
        val expectedKind = when (command) {
            is TransactionCreateCommand.Income -> TransactionKind.INCOME
            is TransactionCreateCommand.Expense -> TransactionKind.EXPENSE
            is TransactionCreateCommand.InterestIncome -> TransactionKind.INTEREST_INCOME
            is TransactionCreateCommand.Transfer -> TransactionKind.TRANSFER
        }
        return mapResponse(api.request("/v1/transactions", HttpMethod.Post, body, context.headers(key)), 201) {
            readTransactionRecord(it, expectedKind).also { record -> validateCreated(record, command) }
        }
    }

    override suspend fun edit(id: TransactionId, patch: TransactionPatch, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> {
        val body = buildJsonObject {
            patch("amount", patch.amount) { JsonPrimitive(it.value) }
            patch("currency", patch.currency) { JsonPrimitive(it.name) }
            patch("occurred_on", patch.occurredOn) { JsonPrimitive(wireDate(it)) }
            patch("account_id", patch.accountId) { JsonPrimitive(wireId(it)) }
            patch("destination_account_id", patch.destinationAccountId) { JsonPrimitive(wireId(it)) }
            patch("received_amount", patch.receivedAmount) { JsonPrimitive(it.value) }
            patch("received_currency", patch.receivedCurrency) { JsonPrimitive(it.name) }
            patch("category_id", patch.categoryId) { JsonPrimitive(wireId(it)) }
            patch("description", patch.description) { JsonPrimitive(it) }
        }
        return mapResponse(api.request("/v1/transactions/${wireId(id)}", HttpMethod.Patch, body, context.headers(key)), 200) {
            readAcknowledgement(it, id, TransactionState.POSTED)
        }
    }

    override suspend fun void(id: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> =
        mapResponse(api.request("/v1/transactions/${wireId(id)}/void", HttpMethod.Post,
            JsonObject(emptyMap()), context.headers(key)), 200) {
            readAcknowledgement(it, id, TransactionState.VOIDED)
        }
}

private fun JsonObjectBuilder.putNullableId(name: String, value: String?) { put(name, value?.let(::JsonPrimitive) ?: JsonNull) }
private fun JsonObjectBuilder.putNullableString(name: String, value: String?) { put(name, value?.let(::JsonPrimitive) ?: JsonNull) }

private fun validateCreated(record: TransactionRecord, command: TransactionCreateCommand) {
    val transaction = record.transaction
    require(transaction.amount == command.amount && transaction.occurredOn == command.occurredOn &&
        transaction.description == command.description) { "Created transaction does not match request" }
    val matches = when (command) {
        is TransactionCreateCommand.Income -> transaction.accountId == command.accountId && transaction.categoryId == command.categoryId
        is TransactionCreateCommand.Expense -> transaction.accountId == command.accountId && transaction.categoryId == command.categoryId
        is TransactionCreateCommand.InterestIncome -> transaction.accountId == command.accountId && transaction.categoryId == command.categoryId
        is TransactionCreateCommand.Transfer -> transaction.accountId == command.accountId &&
            transaction.destinationAccountId == command.destinationAccountId && transaction.receivedAmount == command.receivedAmount &&
            record.quotedRate == command.quotedRate?.let(::canonicalQuote)
    }
    require(matches) { "Created transaction does not match request" }
}

private fun canonicalQuote(value: String): String {
    require(Regex("[0-9]+(?:\\.[0-9]{1,12})?").matches(value)) { "Invalid quoted rate" }
    val whole = value.substringBefore('.').trimStart('0').ifEmpty { "0" }
    val fraction = value.substringAfter('.', "").trimEnd('0')
    return whole + if (fraction.isEmpty()) "" else ".$fraction"
}

private fun readListing(element: JsonElement): Page<TransactionRecord> {
    val root = element.jsonObject
    val items = root.getValue("transactions").jsonArray.map(::readListedTransactionRecord)
    return Page(items, root.nullableString("next_cursor"))
}

private fun readListedTransactionRecord(element: JsonElement): TransactionRecord {
    val root = element.jsonObject
    val kind = parseKind(root.string("kind"))
    val state = parseState(root.string("state"))
    val currency = parseCurrency(root.string("currency"))
    val amount = readMoney(JsonPrimitive(root.string("amount")), currency)
    val accountText = root.nullableString("account_id")
    val categoryText = root.nullableString("category_id")
    val description = root.nullableString("description")
    val destination = root.nullableString("destination_account_id")
    val receivedAmountText = root.nullableString("received_amount")
    val receivedCurrencyText = root.nullableString("received_currency")
    val receivedAmount = if (receivedAmountText == null && receivedCurrencyText == null) null else {
        require(receivedAmountText != null && receivedCurrencyText != null) { "Incomplete transfer amount" }
        readMoney(JsonPrimitive(receivedAmountText), parseCurrency(receivedCurrencyText))
    }
    val transaction = Transaction(
        id = transactionId(root.string("id")), kind = kind, state = state, amount = amount,
        occurredOn = businessDate(root.string("occurred_on")), accountId = accountText?.let(::accountId),
        categoryId = categoryText?.let(::categoryId), description = description,
        destinationAccountId = destination?.let(::accountId), receivedAmount = receivedAmount,
    )
    val (quote, comparison) = readQuoteMetadata(root, kind, receivedAmount?.currency)
    return TransactionRecord(transaction, quote, comparison)
}

private fun readTransactionRecord(element: JsonElement, expectedKind: TransactionKind): TransactionRecord {
    val root = element.jsonObject
    val kind = parseKind(root.string("kind"))
    require(kind == expectedKind) { "Created transaction kind does not match request" }
    require(root.string("state") == "posted") { "Created transaction must be posted" }
    val currency = parseCurrency(root.string("currency"))
    val amount = readMoney(JsonPrimitive(root.string("amount")), currency)
    val accountText = root.nullableString("account_id")
    val categoryText = root.nullableString("category_id")
    val description = root.nullableString("description")
    var destination: AccountId? = null
    var received: Money? = null
    if (kind == TransactionKind.TRANSFER) {
        destination = accountId(root.string("destination_account_id"))
        received = readMoney(JsonPrimitive(root.string("received_amount")), parseCurrency(root.string("received_currency")))
    }
    val transaction = Transaction(
        id = transactionId(root.string("id")), kind = kind, state = TransactionState.POSTED, amount = amount,
        occurredOn = businessDate(root.string("occurred_on")), accountId = accountText?.let(::accountId),
        categoryId = categoryText?.let(::categoryId), description = description,
        destinationAccountId = destination, receivedAmount = received,
    )
    val (quote, comparison) = readQuoteMetadata(root, kind, received?.currency)
    return TransactionRecord(transaction, quote, comparison)
}

private fun readQuoteMetadata(root: JsonObject, kind: TransactionKind, receivedCurrency: Currency?): Pair<String?, RateComparison?> {
    val quotePresent = root.containsKey("quoted_rate")
    val comparisonPresent = root.containsKey("rate_comparison")
    require(quotePresent == comparisonPresent) { "Incomplete quoted-rate data" }
    val quote = if (quotePresent) root.string("quoted_rate").also { value ->
        require(Regex("^(0|[1-9][0-9]*)(\\.[0-9]{0,11}[1-9])?$").matches(value) && value.any { it in '1'..'9' }) {
            "Invalid quoted rate"
        }
    } else null
    val comparison = if (comparisonPresent) {
        require(kind == TransactionKind.TRANSFER && receivedCurrency != null) { "Quote data requires a transfer" }
        readRateComparison(root.getValue("rate_comparison"), receivedCurrency)
    } else null
    require((quote != null) == (comparison != null)) {
        "Invalid quoted-rate data"
    }
    return quote to comparison
}

private fun readRateComparison(element: JsonElement, currency: Currency): RateComparison {
    val root = element.jsonObject
    val indicative = readMoney(JsonPrimitive(root.string("indicative_received_amount")), currency)
    val difference = readMoney(JsonPrimitive(root.string("difference")), currency)
    val mismatchToken = root.getValue("mismatch").jsonPrimitive
    require(!mismatchToken.isString && mismatchToken.booleanOrNull != null) { "Invalid comparison boolean" }
    val mismatch = mismatchToken.boolean
    require(mismatch == (difference.minorUnits != 0L)) { "Invalid rate comparison mismatch" }
    return RateComparison(indicative, difference, mismatch)
}

private fun readAcknowledgement(element: JsonElement, expectedId: TransactionId, expectedState: TransactionState): TransactionAcknowledgement {
    val root = element.jsonObject
    val acknowledgement = TransactionAcknowledgement(transactionId(root.string("id")), parseState(root.string("state")))
    require(acknowledgement.id == expectedId && acknowledgement.state == expectedState) {
        "Transaction acknowledgement does not match request"
    }
    return acknowledgement
}

private fun parseKind(value: String): TransactionKind = when (value) {
    "income" -> TransactionKind.INCOME
    "expense" -> TransactionKind.EXPENSE
    "transfer" -> TransactionKind.TRANSFER
    "opening_balance" -> TransactionKind.OPENING_BALANCE
    "interest_income" -> TransactionKind.INTEREST_INCOME
    else -> throw IllegalArgumentException("Invalid transaction kind")
}
private fun parseState(value: String): TransactionState = when (value) {
    "posted" -> TransactionState.POSTED
    "voided" -> TransactionState.VOIDED
    else -> throw IllegalArgumentException("Invalid transaction state")
}
private fun parseCurrency(value: String): Currency = try { Currency.valueOf(value) }
catch (_: IllegalArgumentException) { throw IllegalArgumentException("Invalid transaction currency") }
