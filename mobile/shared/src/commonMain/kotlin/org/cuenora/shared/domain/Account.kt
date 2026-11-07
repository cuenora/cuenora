package org.cuenora.shared.domain

// Opaque UUID bits supplied by outer layers, without generation or wire parsing.
data class AccountId(val highBits: Long, val lowBits: Long)
data class OwnerId(val highBits: Long, val lowBits: Long)
data class TransactionId(val highBits: Long, val lowBits: Long)
data class CategoryId(val highBits: Long, val lowBits: Long)

enum class AccountType { NORMAL, SAVINGS }
enum class AccountStatus { ACTIVE, ARCHIVED }

class AccountName(input: String) {
    val value: String = input.trim(::isDomainWhitespace)

    init {
        require(value.isNotEmpty()) { "Account name must not be empty" }
        var scalars = 0
        var index = 0
        while (index < value.length) {
            val character = value[index++]
            require(character.code > 31 && character.code != 127) { "Account name contains a control character" }
            if (character in '\uD800'..'\uDBFF') {
                require(index < value.length && value[index] in '\uDC00'..'\uDFFF') { "Invalid account name Unicode" }
                index++
            } else {
                require(character !in '\uDC00'..'\uDFFF') { "Invalid account name Unicode" }
            }
            scalars++
        }
        require(scalars <= 255) { "Account name must contain at most 255 Unicode scalars" }
    }

    override fun equals(other: Any?): Boolean = other is AccountName && value == other.value
    override fun hashCode(): Int = value.hashCode()
}

// Match Rust str::trim, not Kotlin's broader isWhitespace definition.
private fun isDomainWhitespace(c: Char): Boolean = c in '\u0009'..'\u000D' ||
    c == ' ' || c == '\u0085' || c == '\u00A0' || c == '\u1680' ||
    c in '\u2000'..'\u200A' || c == '\u2028' || c == '\u2029' ||
    c == '\u202F' || c == '\u205F' || c == '\u3000'

/** Metadata only. Ownership here is identity, never proof of authorization. */
data class Account(
    val id: AccountId,
    val ownerId: OwnerId,
    val name: AccountName,
    val type: AccountType,
    val currency: Currency,
    val status: AccountStatus,
)
