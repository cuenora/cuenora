package org.cuenora.shared.recurrence

import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*

/** Opaque recurrence identities; cursor and identity strings are never inferred from each other. */
data class RuleId(val highBits: Long, val lowBits: Long)
data class OccurrenceId(val highBits: Long, val lowBits: Long)

enum class RuleState { ACTIVE, PAUSED }
enum class OccurrenceState { PLANNED, CONFIRMED, SKIPPED }

data class RecurrenceRule(
    val id: RuleId,
    val amount: Money,
    val cadence: String,
    val startsOn: BusinessDate,
    val endsOn: BusinessDate?,
    val timeZone: String,
    val accountId: AccountId?,
    val categoryId: CategoryId?,
    val description: String?,
    val state: RuleState,
    val pausedOn: BusinessDate?,
    val generatedThrough: BusinessDate?,
)

data class RecurrenceOccurrence(
    val id: OccurrenceId,
    val ruleId: RuleId,
    val scheduledOn: BusinessDate,
    val dueOn: BusinessDate,
    val state: OccurrenceState,
    val amount: Money,
    val accountId: AccountId?,
    val categoryId: CategoryId?,
    val description: String?,
    val transactionId: TransactionId?,
)
data class OccurrenceActionResult(val id: OccurrenceId, val scheduledOn: BusinessDate, val dueOn: BusinessDate, val state: OccurrenceState)

data class RecurrenceRuleDraft(
    val amount: String,
    val currency: Currency,
    val cadence: String,
    val startsOn: String,
    val endsOn: String? = null,
    val timeZone: String,
    val accountId: AccountId? = null,
    val categoryId: CategoryId? = null,
    val description: String? = null,
)

data class ActualPaymentDraft(
    val amount: String,
    val currency: Currency,
    val occurredOn: String,
    val accountId: AccountId? = null,
    val categoryId: CategoryId? = null,
    val description: String? = null,
)

/** An immutable, normalized review item. It contains no credentials or generated retry key. */
sealed interface RecurrenceReview {
    data class CreateRule(val terms: RecurrenceRuleDraft) : RecurrenceReview
    data class EditRule(val id: RuleId, val terms: RecurrenceRuleDraft) : RecurrenceReview
    data class PauseRule(val id: RuleId) : RecurrenceReview
    data class ResumeRule(val id: RuleId) : RecurrenceReview
    data class Confirm(val id: OccurrenceId, val actual: ActualPaymentDraft) : RecurrenceReview
    data class Skip(val id: OccurrenceId) : RecurrenceReview
    data class Postpone(val id: OccurrenceId, val dueOn: String) : RecurrenceReview
}

/** Persist only idempotent commands; immutable JSON preserves the original retry bytes and key. */
class PendingRecurrence(
    val owner: OwnerId,
    val origin: String,
    val key: IdempotencyKey,
    val command: String,
) {
    override fun toString() = "PendingRecurrence([REDACTED])"
    fun encode(): String = buildJsonObject {
        put("version", 1)
        put("owner_high", owner.highBits.toString()); put("owner_low", owner.lowBits.toString())
        put("origin", origin); put("key", key.value); put("command", command)
    }.toString()
    companion object {
        fun decode(encoded: String): PendingRecurrence {
            val root = Json.parseToJsonElement(encoded).jsonObject
            require(root.getValue("version").jsonPrimitive.int == 1)
            return PendingRecurrence(
                OwnerId(root.string("owner_high").toLong(), root.string("owner_low").toLong()),
                root.string("origin"), IdempotencyKey(root.string("key")), root.string("command"),
            )
        }
    }
}

class RecurrenceJournalException : Exception("Recurrence journal unavailable")
interface RecurrenceJournal {
    suspend fun read(owner: OwnerId, origin: String): PendingRecurrence?
    suspend fun save(owner: OwnerId, origin: String, pending: PendingRecurrence)
    suspend fun clear(owner: OwnerId, origin: String)
}
