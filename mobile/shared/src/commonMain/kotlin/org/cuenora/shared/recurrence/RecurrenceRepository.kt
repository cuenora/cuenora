package org.cuenora.shared.recurrence

import io.ktor.client.request.*
import io.ktor.http.*
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*

interface RecurrenceRepository {
    suspend fun rules(limit: Int = 50, cursor: String? = null, state: RuleState? = null): RepositoryResult<Page<RecurrenceRule>>
    suspend fun occurrences(limit: Int = 50, cursor: String? = null, state: OccurrenceState? = null, ruleId: RuleId? = null): RepositoryResult<Page<RecurrenceOccurrence>>
    suspend fun create(command: RecurrenceRuleDraft, key: IdempotencyKey): RepositoryResult<RecurrenceRule>
    suspend fun edit(id: RuleId, command: RecurrenceRuleDraft, current: RecurrenceRule): RepositoryResult<RecurrenceRule>
    suspend fun pause(id: RuleId): RepositoryResult<RecurrenceRule>
    suspend fun resume(id: RuleId): RepositoryResult<RecurrenceRule>
    suspend fun confirm(id: OccurrenceId, actual: ActualPaymentDraft, key: IdempotencyKey): RepositoryResult<TransactionRecord>
    suspend fun skip(id: OccurrenceId, key: IdempotencyKey): RepositoryResult<OccurrenceActionResult>
    suspend fun postpone(id: OccurrenceId, dueOn: BusinessDate, key: IdempotencyKey): RepositoryResult<OccurrenceActionResult>
}

/** HTTP adapter for the existing recurrence mutations and additive listing endpoints. */
class HttpRecurrenceRepository(private val api: ApiClient, private val context: RepositoryContext) : RecurrenceRepository {
    override suspend fun rules(limit: Int, cursor: String?, state: RuleState?) =
        mapResponse(api.request(queryPath("/v1/recurring-rules", listOf(
            "limit" to limit.also { require(it in 1..100) }.toString(), "cursor" to cursor,
            "state" to state?.name?.lowercase(),
        )), headers = context.headers()), 200) { element ->
            val root = element.jsonObject
            require(root.keys.containsAll(setOf("rules", "next_cursor")))
            Page(root.getValue("rules").jsonArray.map(::decodeRule), root.nullableString("next_cursor"))
        }

    override suspend fun occurrences(limit: Int, cursor: String?, state: OccurrenceState?, ruleId: RuleId?) =
        mapResponse(api.request(queryPath("/v1/recurring-occurrences", listOf(
            "limit" to limit.also { require(it in 1..100) }.toString(), "cursor" to cursor,
            "state" to state?.name?.lowercase(), "rule_id" to ruleId?.let(::wireId),
        )), headers = context.headers()), 200) { element ->
            val root = element.jsonObject
            require(root.keys.containsAll(setOf("occurrences", "next_cursor")))
            Page(root.getValue("occurrences").jsonArray.map(::decodeOccurrence), root.nullableString("next_cursor"))
        }

    override suspend fun create(command: RecurrenceRuleDraft, key: IdempotencyKey) = mapResponse(
        api.request("/v1/recurring-rules", HttpMethod.Post, terms(command), context.headers(key)), 201, ::decodeRule)

    override suspend fun edit(id: RuleId, command: RecurrenceRuleDraft, current: RecurrenceRule): RepositoryResult<RecurrenceRule> {
        val body = buildJsonObject {
            val amount = readMoney(JsonPrimitive(command.amount), command.currency)
            if (amount != current.amount) put("amount", writeMoney(amount))
            if (command.currency != current.amount.currency) put("currency", command.currency.name)
            if (command.cadence != current.cadence) put("cadence", command.cadence)
            if (businessDate(command.startsOn) != current.startsOn) put("starts_on", command.startsOn)
            if (command.timeZone != current.timeZone) put("time_zone", command.timeZone)
            if (command.endsOn?.let(::businessDate) != current.endsOn) put("ends_on", command.endsOn?.let(::JsonPrimitive) ?: JsonNull)
            // An unchanged archived account is deliberately omitted, never explicitly reselected.
            if (command.accountId != current.accountId) put("account_id", command.accountId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
            if (command.categoryId != current.categoryId) put("category_id", command.categoryId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
            if (command.description != current.description) put("description", command.description?.let(::JsonPrimitive) ?: JsonNull)
        }
        if (body.isEmpty()) return RepositoryResult.Success(current)
        return mapResponse(api.request("/v1/recurring-rules/${wireId(id)}", HttpMethod.Patch, body, context.headers()), 200, ::decodeRule)
    }

    override suspend fun pause(id: RuleId) = mapResponse(
        api.request("/v1/recurring-rules/${wireId(id)}/pause", HttpMethod.Post, buildJsonObject {}, context.headers()), 200, ::decodeRule)
    override suspend fun resume(id: RuleId) = mapResponse(
        api.request("/v1/recurring-rules/${wireId(id)}/resume", HttpMethod.Post, buildJsonObject {}, context.headers()), 200, ::decodeRule)

    override suspend fun confirm(id: OccurrenceId, actual: ActualPaymentDraft, key: IdempotencyKey) = mapResponse(
        api.request("/v1/recurring-occurrences/${wireId(id)}/confirm", HttpMethod.Post, buildJsonObject {
            put("amount", writeMoney(readMoney(JsonPrimitive(actual.amount), actual.currency)))
            put("currency", actual.currency.name); put("occurred_on", actual.occurredOn)
            putNullableId("account_id", actual.accountId?.let(::wireId))
            putNullableId("category_id", actual.categoryId?.let(::wireId))
            put("description", actual.description?.let(::JsonPrimitive) ?: JsonNull)
        }, context.headers(key)), 201) { readExpense(it, actual) }

    override suspend fun skip(id: OccurrenceId, key: IdempotencyKey) = mapResponse(
        api.request("/v1/recurring-occurrences/${wireId(id)}/skip", HttpMethod.Post, buildJsonObject {}, context.headers(key)), 200, ::decodeAction)

    override suspend fun postpone(id: OccurrenceId, dueOn: BusinessDate, key: IdempotencyKey) = mapResponse(
        api.request("/v1/recurring-occurrences/${wireId(id)}/postpone", HttpMethod.Post,
            buildJsonObject { put("due_on", wireDate(dueOn)) }, context.headers(key)), 200, ::decodeAction)

    private fun terms(command: RecurrenceRuleDraft) = buildJsonObject {
        put("amount", writeMoney(readMoney(JsonPrimitive(command.amount), command.currency)))
        put("currency", command.currency.name); put("cadence", command.cadence); put("starts_on", command.startsOn)
        put("ends_on", command.endsOn?.let(::JsonPrimitive) ?: JsonNull); put("time_zone", command.timeZone)
        putNullableId("account_id", command.accountId?.let(::wireId)); putNullableId("category_id", command.categoryId?.let(::wireId))
        put("description", command.description?.let(::JsonPrimitive) ?: JsonNull)
    }

    private fun decodeRule(value: JsonElement): RecurrenceRule {
        val root = value.jsonObject
        val currency = Currency.valueOf(root.string("currency"))
        return RecurrenceRule(
            ruleId(root.string("id")), readMoney(root.getValue("amount"), currency), root.string("cadence"),
            businessDate(root.string("starts_on")), root.nullableString("ends_on")?.let(::businessDate),
            root.string("time_zone"), root.nullableString("account_id")?.let(::accountId),
            root.nullableString("category_id")?.let(::categoryId), root.nullableString("description"),
            RuleState.valueOf(root.string("state").uppercase()), root.nullableString("paused_on")?.let(::businessDate),
            root.nullableString("generated_through")?.let(::businessDate),
        )
    }

    private fun decodeOccurrence(value: JsonElement): RecurrenceOccurrence {
        val root = value.jsonObject
        val currency = Currency.valueOf(root.string("currency"))
        return RecurrenceOccurrence(
            occurrenceId(root.string("id")), ruleId(root.string("rule_id")), businessDate(root.string("scheduled_on")),
            businessDate(root.string("due_on")), OccurrenceState.valueOf(root.string("state").uppercase()),
            readMoney(root.getValue("amount"), currency), root.nullableString("account_id")?.let(::accountId),
            root.nullableString("category_id")?.let(::categoryId), root.nullableString("description"),
            root.nullableString("transaction_id")?.let(::transactionId),
        )
    }

    private fun decodeAction(value: JsonElement): OccurrenceActionResult {
        val root = value.jsonObject
        return OccurrenceActionResult(occurrenceId(root.string("id")), businessDate(root.string("scheduled_on")),
            businessDate(root.string("due_on")), OccurrenceState.valueOf(root.string("state").uppercase()))
    }

    private fun readExpense(value: JsonElement, expected: ActualPaymentDraft): TransactionRecord {
        val root = value.jsonObject
        require(root.string("kind") == "expense" && root.string("state") == "posted")
        val currency = Currency.valueOf(root.string("currency"))
        val transaction = Transaction(transactionId(root.string("id")), TransactionKind.EXPENSE, TransactionState.POSTED,
            readMoney(JsonPrimitive(root.string("amount")), currency), businessDate(root.string("occurred_on")),
            root.nullableString("account_id")?.let(::accountId), root.nullableString("category_id")?.let(::categoryId),
            root.nullableString("description"))
        require(transaction.amount == readMoney(JsonPrimitive(expected.amount), expected.currency) &&
            transaction.occurredOn == businessDate(expected.occurredOn) && transaction.accountId == expected.accountId &&
            transaction.categoryId == expected.categoryId && transaction.description == expected.description) {
            "Confirmed expense does not match reviewed terms"
        }
        return TransactionRecord(transaction)
    }

    internal fun decodeCreatedRule(value: JsonElement) = decodeRule(value)
    internal fun decodeConfirmedExpense(value: JsonElement, expected: ActualPaymentDraft) = readExpense(value, expected)
    internal fun decodeActionResult(value: JsonElement) = decodeAction(value)
}

private fun JsonObjectBuilder.putNullableId(name: String, value: String?) = put(name, value?.let(::JsonPrimitive) ?: JsonNull)
private fun ruleId(text: String) = uuidBits(text).let { RuleId(it.first, it.second) }
private fun occurrenceId(text: String) = uuidBits(text).let { OccurrenceId(it.first, it.second) }
private fun uuidText(high: Long, low: Long): String {
    val digits = high.toULong().toString(16).padStart(16, '0') + low.toULong().toString(16).padStart(16, '0')
    return "${digits.take(8)}-${digits.substring(8, 12)}-${digits.substring(12, 16)}-${digits.substring(16, 20)}-${digits.drop(20)}"
}
private fun wireId(id: RuleId): String = uuidText(id.highBits, id.lowBits)
private fun wireId(id: OccurrenceId): String = uuidText(id.highBits, id.lowBits)
