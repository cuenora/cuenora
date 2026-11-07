package org.cuenora.shared.repository

import io.ktor.http.*
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.*
import kotlinx.coroutines.CancellationException
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.serialization.MoneyAmountSerializer

sealed interface RepositoryResult<out T> {
    data class Success<T>(val value: T) : RepositoryResult<T>
    data class Failure(val error: ApiFailure) : RepositoryResult<Nothing>
}

/** One explicit user session snapshot. Replace repositories when the user/session changes. */
class RepositoryContext(val ownerId: OwnerId, private val accessToken: String) {
    init {
        require(accessToken.isNotEmpty() && accessToken.all {
            it in 'A'..'Z' || it in 'a'..'z' || it in '0'..'9' || it in "-._~+/="
        }) { "Invalid bearer credential" }
    }
    internal fun headers(key: IdempotencyKey? = null): Map<String, String> = buildMap {
        put(HttpHeaders.Authorization, "Bearer $accessToken")
        if (key != null) put("Idempotency-Key", key.value)
    }
    override fun toString(): String = "RepositoryContext([REDACTED])"
}

/** Generated/persisted by the caller for one logical mutation, never by a repository. */
class IdempotencyKey(internal val value: String) {
    init { require(Regex("[A-Za-z0-9_-]{1,128}").matches(value)) { "Invalid idempotency key" } }
    override fun toString(): String = "IdempotencyKey([REDACTED])"
    override fun equals(other: Any?): Boolean = other is IdempotencyKey && value == other.value
    override fun hashCode(): Int = value.hashCode()
}

data class Page<T>(val items: List<T>, val nextCursor: String?)

/** A currencyless decimal PATCH field, not recorded Money or an FX value. */
data class DecimalAmount(val value: String) {
    init {
        // The field serializer uses currency only as an output tag. Reuse its exact
        // scale/range checks; no currency is attached to this transport scalar.
        readMoney(JsonPrimitive(value), Currency.SEK)
    }
}

/** Nullable PATCH fields distinguish preserve, clear and replace. */
sealed interface PatchField<out T> {
    data object Absent : PatchField<Nothing>
    data object Null : PatchField<Nothing>
    data class Value<T>(val value: T) : PatchField<T>
}

internal fun <T> mapResponse(result: ApiResult, expectedStatus: Int, decode: (JsonElement) -> T): RepositoryResult<T> =
    when (result) {
        is ApiResult.Failure -> RepositoryResult.Failure(result.error)
        is ApiResult.Success -> {
            fun invalid() = RepositoryResult.Failure(ApiFailure.InvalidResponse(result.status, result.requestId))
            if (result.status != expectedStatus || result.body == null) invalid()
            else try { RepositoryResult.Success(decode(result.body)) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (_: IllegalArgumentException) { invalid() }
            catch (_: SerializationException) { invalid() }
            catch (_: IllegalStateException) { invalid() }
            catch (_: NoSuchElementException) { invalid() }
        }
    }

internal fun JsonObject.string(name: String): String {
    val value = getValue(name) as? JsonPrimitive ?: throw IllegalArgumentException("Invalid resource field")
    require(value.isString) { "Invalid resource field" }
    return value.content
}

internal fun JsonObject.nullableString(name: String): String? {
    require(containsKey(name)) { "Missing resource field" }
    return if (getValue(name) == JsonNull) null else string(name)
}

internal fun JsonObject.optionalString(name: String): String? =
    if (containsKey(name)) nullableString(name) else null

internal fun readMoney(value: JsonElement, currency: Currency): Money =
    Json.decodeFromJsonElement(MoneyAmountSerializer(currency), value)

internal fun writeMoney(value: Money): JsonElement =
    Json.encodeToJsonElement(MoneyAmountSerializer(value.currency), value)

private val uuidPattern = Regex("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")

internal fun uuidBits(text: String): Pair<Long, Long> {
    require(uuidPattern.matches(text)) { "Invalid resource identity" }
    val digits = text.replace("-", "")
    return digits.take(16).toULong(16).toLong() to digits.drop(16).toULong(16).toLong()
}
internal fun accountId(text: String): AccountId = uuidBits(text).let { AccountId(it.first, it.second) }
internal fun transactionId(text: String): TransactionId = uuidBits(text).let { TransactionId(it.first, it.second) }
internal fun categoryId(text: String): CategoryId = uuidBits(text).let { CategoryId(it.first, it.second) }
private fun uuidText(high: Long, low: Long): String {
    val digits = high.toULong().toString(16).padStart(16, '0') + low.toULong().toString(16).padStart(16, '0')
    return "${digits.take(8)}-${digits.substring(8, 12)}-${digits.substring(12, 16)}-${digits.substring(16, 20)}-${digits.drop(20)}"
}
internal fun wireId(id: AccountId): String = uuidText(id.highBits, id.lowBits)
internal fun wireId(id: TransactionId): String = uuidText(id.highBits, id.lowBits)
internal fun wireId(id: CategoryId): String = uuidText(id.highBits, id.lowBits)
internal fun wireDate(date: BusinessDate): String =
    "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"
internal fun businessDate(text: String): BusinessDate {
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(text)) { "Invalid resource date" }
    return BusinessDate(text.take(4).toInt(), text.substring(5, 7).toInt(), text.takeLast(2).toInt())
}

internal fun queryPath(path: String, parameters: List<Pair<String, String?>>): String {
    val encoded = parameters.mapNotNull { (name, value) -> value?.let { name to it } }.formUrlEncode()
    return if (encoded.isEmpty()) path else "$path?$encoded"
}

internal fun <T> JsonObjectBuilder.patch(name: String, field: PatchField<T>, encode: (T) -> JsonElement) {
    when (field) {
        PatchField.Absent -> Unit
        PatchField.Null -> put(name, JsonNull)
        is PatchField.Value -> put(name, encode(field.value))
    }
}
