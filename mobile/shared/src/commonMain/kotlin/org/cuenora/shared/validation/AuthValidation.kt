package org.cuenora.shared.validation

/** Local, deterministic checks for authentication form input; the server remains authoritative. */
fun validateRegistrationForm(email: String, password: String): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    if (!isAdvisoryEmail(email)) issues += ValidationIssue("email", "invalid_email")
    if (!isSignupPassword(password)) issues += passwordIssue(password)
    return FormValidation(issues)
}

/** Login intentionally accepts any password within the backend's input bounds. */
fun validateLoginForm(email: String, password: String): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    if (!isAdvisoryEmail(email)) issues += ValidationIssue("email", "invalid_email")
    if (!isWellFormedUnicode(password)) {
        issues += ValidationIssue("password", "invalid_password_unicode")
    } else if (password.encodeToByteArray().size > LOGIN_PASSWORD_MAX_BYTES) {
        issues += ValidationIssue("password", "password_too_long")
    } else if (unicodeScalarCount(password)!! > PASSWORD_MAX_SCALARS) {
        issues += ValidationIssue("password", "invalid_password_length")
    }
    return FormValidation(issues)
}

fun validateRecoveryEmailForm(email: String): FormValidation =
    if (isAdvisoryEmail(email)) FormValidation(emptyList())
    else FormValidation(listOf(ValidationIssue("email", "invalid_email")))

fun validatePasswordResetForm(token: String, newPassword: String): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    if (!hasVersionedHexToken(token, RECOVERY_TOKEN_PREFIX)) {
        issues += ValidationIssue("token", "invalid_token")
    }
    if (!isSignupPassword(newPassword)) issues += passwordIssue(newPassword)
    return FormValidation(issues)
}

fun validateEmailVerificationToken(token: String): FormValidation =
    if (hasVersionedHexToken(token, VERIFICATION_TOKEN_PREFIX)) FormValidation(emptyList())
    else FormValidation(listOf(ValidationIssue("token", "invalid_token")))

private const val EMAIL_MAX_UTF8_BYTES = 1024
private const val LOCAL_PART_MAX_BYTES = 64
private const val PASSWORD_MIN_SCALARS = 15
private const val PASSWORD_MAX_SCALARS = 128
private const val LOGIN_PASSWORD_MAX_BYTES = 512
private const val RECOVERY_TOKEN_PREFIX = "pr1_"
private const val VERIFICATION_TOKEN_PREFIX = "ev1_"
private const val TOKEN_HEX_DIGITS = 64

private fun passwordIssue(password: String): ValidationIssue =
    if (!isWellFormedUnicode(password)) {
        ValidationIssue("password", "invalid_password_unicode")
    } else {
        ValidationIssue("password", "invalid_password_length")
    }

private fun isSignupPassword(password: String): Boolean {
    val scalarCount = unicodeScalarCount(password) ?: return false
    return scalarCount in PASSWORD_MIN_SCALARS..PASSWORD_MAX_SCALARS
}

/**
 * Check only mailbox facts that can be evaluated without IDNA conversion. In particular,
 * do not apply an ASCII-domain byte limit to a Unicode domain before IDNA processing.
 */
private fun isAdvisoryEmail(input: String): Boolean {
    if (!isWellFormedUnicode(input) || input.encodeToByteArray().size > EMAIL_MAX_UTF8_BYTES) return false
    if (input.any { it.isWhitespace() || it.isControlCharacter() }) return false

    val separator = input.indexOf('@')
    if (separator <= 0 || separator != input.lastIndexOf('@')) return false
    val local = input.substring(0, separator)
    val domain = input.substring(separator + 1)

    if (local.length > LOCAL_PART_MAX_BYTES || local.startsWith('.') || local.endsWith('.') || ".." in local) {
        return false
    }
    if (!local.all { it.isAsciiAlphaNumeric() || it in ".!#$%&'*+-/=?^_`{|}~" }) return false

    if (domain.isEmpty()) return false
    // IDNA recognizes these three Unicode dot variants as label separators. Map only for
    // this advisory structure check; preserve the submitted domain for the server.
    val structureDomain = buildString(domain.length) {
        domain.forEach { character ->
            append(if (character.isIdnaMappedLabelSeparator()) '.' else character)
        }
    }
    if (structureDomain.endsWith('.')) return false
    val labels = structureDomain.split('.')
    if (labels.size < 2) return false
    return labels.all { label ->
        label.isNotEmpty() &&
            !label.startsWith('-') &&
            !label.endsWith('-') &&
            label.all { character ->
                character.isAsciiAlphaNumeric() || character == '-' ||
                    (!character.isAscii() && !character.isWhitespace() && !character.isControlCharacter())
            }
    }
}

private fun hasVersionedHexToken(value: String, prefix: String): Boolean =
    value.length == prefix.length + TOKEN_HEX_DIGITS &&
        value.startsWith(prefix) &&
        value.substring(prefix.length).all { it.isLowercaseHexDigit() }

private fun Char.isLowercaseHexDigit(): Boolean = this in '0'..'9' || this in 'a'..'f'

private fun Char.isAsciiAlphaNumeric(): Boolean =
    this in 'a'..'z' || this in 'A'..'Z' || this in '0'..'9'

private fun Char.isAscii(): Boolean = code <= 0x7f

private fun Char.isIdnaMappedLabelSeparator(): Boolean =
    this == '\u3002' || this == '\uFF0E' || this == '\uFF61'

private fun Char.isControlCharacter(): Boolean = code < 0x20 || code in 0x7f..0x9f

/** Return the Unicode scalar count, or null when the UTF-16 contains an unpaired surrogate. */
private fun unicodeScalarCount(value: String): Int? {
    var count = 0
    var index = 0
    while (index < value.length) {
        val current = value[index]
        when {
            current.isHighSurrogate() -> {
                if (index + 1 >= value.length || !value[index + 1].isLowSurrogate()) return null
                index += 2
            }
            current.isLowSurrogate() -> return null
            else -> index++
        }
        count++
    }
    return count
}

private fun isWellFormedUnicode(value: String): Boolean = unicodeScalarCount(value) != null
