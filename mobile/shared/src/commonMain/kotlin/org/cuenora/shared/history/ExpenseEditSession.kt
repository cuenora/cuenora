package org.cuenora.shared.history

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.serialization.json.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*

enum class ExpenseEditResult { Saved, Pending, Rejected, Unavailable }

data class PendingExpenseEdit(
    val owner: OwnerId,
    val target: TransactionId,
    val key: IdempotencyKey,
    val patch: TransactionPatch,
) {
    init {
        require(listOf(patch.currency, patch.accountId, patch.destinationAccountId, patch.receivedAmount,
            patch.receivedCurrency).all { it == PatchField.Absent })
    }

    override fun toString() = "PendingExpenseEdit([REDACTED])"

    internal fun encode(): String = buildJsonObject {
        put("ownerHigh", owner.highBits); put("ownerLow", owner.lowBits)
        put("targetHigh", target.highBits); put("targetLow", target.lowBits)
        put("key", key.value)
        field("amount", patch.amount) { JsonPrimitive(it.value) }
        field("date", patch.occurredOn) { JsonPrimitive(wireDate(it)) }
        field("category", patch.categoryId) {
            JsonArray(listOf(JsonPrimitive(it.highBits), JsonPrimitive(it.lowBits)))
        }
        field("description", patch.description, ::JsonPrimitive)
    }.toString()

    companion object {
        internal fun decode(raw: String): PendingExpenseEdit {
            val json = Json.parseToJsonElement(raw).jsonObject
            require(json.keys.all { it in setOf("ownerHigh", "ownerLow", "targetHigh", "targetLow",
                "key", "amount", "date", "category", "description") })
            fun number(name: String) = json.getValue(name).jsonPrimitive.long
            return PendingExpenseEdit(
                OwnerId(number("ownerHigh"), number("ownerLow")),
                TransactionId(number("targetHigh"), number("targetLow")),
                IdempotencyKey(json.getValue("key").jsonPrimitive.content),
                TransactionPatch(
                    amount = json.field("amount") { DecimalAmount(it.jsonPrimitive.content) },
                    occurredOn = json.field("date") { element ->
                        val parts = element.jsonPrimitive.content.split('-')
                        require(parts.size == 3)
                        BusinessDate(parts[0].toInt(), parts[1].toInt(), parts[2].toInt())
                    },
                    categoryId = json.field("category") {
                        val bits = it.jsonArray
                        require(bits.size == 2)
                        CategoryId(bits[0].jsonPrimitive.long, bits[1].jsonPrimitive.long)
                    },
                    description = json.field("description") { it.jsonPrimitive.content },
                ),
            )
        }
    }
}

private fun <T> JsonObjectBuilder.field(name: String, field: PatchField<T>, encode: (T) -> JsonElement) {
    when (field) {
        PatchField.Absent -> Unit
        PatchField.Null -> put(name, JsonNull)
        is PatchField.Value -> put(name, encode(field.value))
    }
}

private fun <T> JsonObject.field(name: String, decode: (JsonElement) -> T): PatchField<T> =
    when (val value = this[name]) {
        null -> PatchField.Absent
        JsonNull -> PatchField.Null
        else -> PatchField.Value(decode(value))
    }

interface ExpenseEditJournal {
    suspend fun read(owner: OwnerId): PendingExpenseEdit?
    suspend fun save(pending: PendingExpenseEdit)
    suspend fun clear(owner: OwnerId)
}

class ExpenseEditSession(
    private val owner: suspend () -> OwnerId?,
    private val write: suspend (PendingExpenseEdit) -> RepositoryResult<TransactionAcknowledgement>,
    private val journal: ExpenseEditJournal,
    private val newKey: () -> IdempotencyKey,
) {
    private var boundOwner: OwnerId? = null
    private var retired = false

    fun clear() { retired = true }

    private suspend fun verifiedOwner(): OwnerId? {
        if (retired) return null
        val current = owner() ?: return null
        if (retired || boundOwner?.let { it != current } == true) return null
        boundOwner = current
        return current
    }

    suspend fun hasPending(): Boolean = checked(false) {
        val current = verifiedOwner() ?: return@checked false
        journal.read(current) != null
    }

    suspend fun save(id: TransactionId, patch: TransactionPatch): ExpenseEditResult = operation {
        val current = verifiedOwner() ?: return@operation ExpenseEditResult.Unavailable
        if (journal.read(current) != null) return@operation ExpenseEditResult.Pending
        if (retired) return@operation ExpenseEditResult.Unavailable
        val pending = PendingExpenseEdit(current, id, newKey(), patch)
        journal.save(pending)
        send(pending, retry = false)
    }

    suspend fun retry(): ExpenseEditResult = operation {
        val current = verifiedOwner() ?: return@operation ExpenseEditResult.Unavailable
        val pending = journal.read(current) ?: return@operation ExpenseEditResult.Unavailable
        send(pending, retry = true)
    }

    private suspend fun send(pending: PendingExpenseEdit, retry: Boolean): ExpenseEditResult {
        if (retired) return ExpenseEditResult.Pending
        return when (val result = write(pending)) {
            is RepositoryResult.Success -> {
                if (result.value.id != pending.target || result.value.state != TransactionState.POSTED) {
                    ExpenseEditResult.Pending
                } else {
                    journal.clear(pending.owner)
                    ExpenseEditResult.Saved
                }
            }
            is RepositoryResult.Failure -> {
                val failure = result.error as? ApiFailure.Http
                val code = failure?.error?.code
                // The server looks up the original key before rejecting a voided target.
                val voided = failure?.status == 409 && code == "transaction_voided"
                val rejectedFirstAttempt = !retry && when (failure?.status) {
                    400 -> code in setOf("invalid_request", "invalid_business_date", "date_out_of_range", "future_business_date")
                    404 -> code == "not_found"
                    409 -> code in setOf("profile_zone_unconfirmed", "account_archived")
                    else -> false
                }
                if (voided || rejectedFirstAttempt) {
                    journal.clear(pending.owner)
                    ExpenseEditResult.Rejected
                } else ExpenseEditResult.Pending
            }
        }
    }

    private suspend fun operation(block: suspend () -> ExpenseEditResult): ExpenseEditResult {
        if (!writeGate.tryLock()) return ExpenseEditResult.Unavailable
        return try { checked(ExpenseEditResult.Pending, block) } finally { writeGate.unlock() }
    }

    private suspend fun <T> checked(fallback: T, block: suspend () -> T): T = try {
        block()
    } catch (cancelled: CancellationException) {
        throw cancelled
    } catch (_: Exception) {
        fallback
    }

    private companion object {
        // The journal can be shared by two presentation instances during Activity recreation.
        val writeGate = Mutex()
    }
}

fun authenticatedExpenseEditSession(
    session: AuthSession,
    api: ApiClient,
    journal: ExpenseEditJournal,
    newKey: () -> IdempotencyKey,
): ExpenseEditSession = ExpenseEditSession(
    owner = {
        val profile = session.authorized { HttpOnboardingRepository(api, it).profile() }
        (profile as? RepositoryResult.Success)?.value?.takeIf { it.emailVerified }?.ownerId
    },
    write = { pending ->
        session.authorized { token ->
            when (val profile = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> profile
                is RepositoryResult.Success -> {
                    if (profile.value.ownerId != pending.owner || !profile.value.emailVerified) {
                        RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                    } else HttpTransactionRepository(api, RepositoryContext(pending.owner, token))
                        .edit(pending.target, pending.patch, pending.key)
                }
            }
        }
    },
    journal = journal,
    newKey = newKey,
)
