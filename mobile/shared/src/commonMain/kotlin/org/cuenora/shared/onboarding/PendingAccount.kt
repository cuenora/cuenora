package org.cuenora.shared.onboarding

import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*

/** A write-ahead record, not an offline draft. Save before POST; retain until acknowledged. */
class PendingAccount(val owner: OwnerId, val command: CreateAccountCommand, val key: IdempotencyKey) {
    override fun toString() = "PendingAccount([REDACTED])"
    fun encode(): String = buildJsonObject {
        put("version", 1)
        put("owner_high", owner.highBits.toString()); put("owner_low", owner.lowBits.toString())
        put("key", key.value)
        put("name", command.name.value); put("type", command.type.name); put("currency", command.currency.name)
        command.openingBalance?.let {
            put("amount", writeMoney(it.amount)); put("date", wireDate(it.occurredOn))
        }
    }.toString()
    companion object {
        fun decode(encoded: String): PendingAccount {
            val root = Json.parseToJsonElement(encoded).jsonObject
            require(root.getValue("version").jsonPrimitive.int == 1)
            val currency = Currency.valueOf(root.string("currency"))
            val opening = if (root.containsKey("amount")) OpeningBalance(
                readMoney(root.getValue("amount"), currency), businessDate(root.string("date"))) else null
            return PendingAccount(OwnerId(root.string("owner_high").toLong(), root.string("owner_low").toLong()),
                CreateAccountCommand(AccountName(root.string("name")), AccountType.valueOf(root.string("type")), currency, opening),
                IdempotencyKey(root.string("key")))
        }
    }
}

class PendingAccountStoreException : Exception("Pending account storage unavailable")
interface PendingAccountStore {
    suspend fun read(owner: OwnerId): PendingAccount?
    suspend fun save(pending: PendingAccount)
    suspend fun clear(owner: OwnerId)
}
