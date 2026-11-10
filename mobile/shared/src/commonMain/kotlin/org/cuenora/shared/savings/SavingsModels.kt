package org.cuenora.shared.savings

import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.*

/** User-reviewed commands; configuration uses a natural date slot, receipts a permanent key. */
sealed interface SavingsAction {
    val account: AccountId
    data class Terms(override val account: AccountId, val command: SavingsTermsCommand) : SavingsAction
    data class Rate(override val account: AccountId, val command: SavingsRateCommand) : SavingsAction
    data class Receipt(override val account: AccountId, val amount: String, val currency: Currency,
        val occurredOn: String, val description: String? = null) : SavingsAction
}
class PendingSavings(val owner: OwnerId, val origin: String, val action: SavingsAction, val key: IdempotencyKey?) {
    init {
        require(origin.isNotBlank() && origin.length <= 1024)
        require((action is SavingsAction.Receipt) == (key != null))
        if (action is SavingsAction.Receipt) require(validateTransactionForm(action.form()).isLocallyValid)
    }
    override fun toString() = "PendingSavings([REDACTED])"
    fun encode(): String = buildJsonObject {
        put("version", 1); put("owner_high", owner.highBits.toString()); put("owner_low", owner.lowBits.toString())
        put("origin", origin); put("key", key?.value?.let(::JsonPrimitive) ?: JsonNull)
        put("account_id", wireId(action.account))
        when (val a = action) {
            is SavingsAction.Terms -> { put("kind", "terms"); put("body", encodeTerms(a.command)) }
            is SavingsAction.Rate -> {
                put("kind", "rate"); put("body", buildJsonObject {
                    put("effective_on", wireDate(a.command.effectiveOn)); put("nominal_annual_percent", a.command.nominalAnnualPercent)
                })
            }
            is SavingsAction.Receipt -> {
                put("kind", "receipt"); put("body", buildJsonObject {
                    put("amount", a.amount); put("currency", a.currency.name); put("occurred_on", a.occurredOn)
                    put("description", a.description?.let(::JsonPrimitive) ?: JsonNull)
                })
            }
        }
    }.toString()
    companion object {
        fun decode(encoded: String): PendingSavings {
            require(encoded.length <= 16000)
            val root = Json.parseToJsonElement(encoded).jsonObject
            require(root.keys == setOf("version", "owner_high", "owner_low", "origin", "key", "account_id", "kind", "body"))
            require(root.getValue("version") == JsonPrimitive(1))
            val body = root.getValue("body").jsonObject
            val account = accountId(root.string("account_id"))
            val action = when (root.string("kind")) {
                "terms" -> {
                    val form = SavingsTermsForm(body.string("effective_on"), body.string("payout_frequency"),
                        body.getValue("capitalize").jsonPrimitive.also { require(!it.isString) }.boolean,
                        body.string("rate_basis"), body.string("day_count"), body.string("balance_timing"), body.string("rounding"))
                    require(validateSavingsTermsForm(form).isLocallyValid && body.keys == encodeTerms(SavingsTermsCommand(businessDate(form.effectiveOn), form.payoutFrequency, form.capitalize!!)).keys)
                    SavingsAction.Terms(account, SavingsTermsCommand(businessDate(form.effectiveOn), form.payoutFrequency, form.capitalize))
                }
                "rate" -> {
                    require(body.keys == setOf("effective_on", "nominal_annual_percent"))
                    SavingsAction.Rate(account, SavingsRateCommand(businessDate(body.string("effective_on")), body.string("nominal_annual_percent")))
                }
                "receipt" -> {
                    require(body.keys == setOf("amount", "currency", "occurred_on", "description"))
                    SavingsAction.Receipt(account, body.string("amount"), Currency.valueOf(body.string("currency")), body.string("occurred_on"), body.nullableString("description"))
                }
                else -> throw IllegalArgumentException("Invalid savings command")
            }
            return PendingSavings(OwnerId(root.string("owner_high").toLong(), root.string("owner_low").toLong()), root.string("origin"), action, root.nullableString("key")?.let(::IdempotencyKey))
        }
    }
}
internal fun SavingsAction.Receipt.form() = TransactionForm(TransactionKind.INTEREST_INCOME, amount, currency, occurredOn, account, description = description)
class SavingsJournalException : Exception("Savings journal unavailable")
interface SavingsJournal {
    suspend fun read(owner: OwnerId, origin: String): PendingSavings?
    suspend fun save(owner: OwnerId, origin: String, pending: PendingSavings)
    suspend fun clear(owner: OwnerId, origin: String)
}
