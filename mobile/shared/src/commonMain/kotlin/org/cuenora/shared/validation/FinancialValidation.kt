package org.cuenora.shared.validation

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.decodeFromJsonElement
import org.cuenora.shared.domain.Account
import org.cuenora.shared.domain.AccountId
import org.cuenora.shared.domain.AccountName
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.Money
import org.cuenora.shared.domain.Transaction
import org.cuenora.shared.domain.TransactionId
import org.cuenora.shared.domain.TransactionKind
import org.cuenora.shared.domain.TransactionState
import org.cuenora.shared.serialization.MoneyAmountSerializer

data class AccountForm(
    val name: String,
    val type: AccountType,
    val currency: Currency,
    val openingBalanceAmount: String? = null,
    val openingBalanceOccurredOn: String? = null,
)

data class TransactionForm(
    val kind: TransactionKind,
    val amount: String,
    val currency: Currency,
    val occurredOn: String,
    val accountId: AccountId? = null,
    val categoryId: CategoryId? = null,
    val description: String? = null,
    val destinationAccountId: AccountId? = null,
    val receivedAmount: String? = null,
    val receivedCurrency: Currency? = null,
    val quotedRate: String? = null,
)

data class RecurringExpenseForm(
    val amount: String,
    val currency: Currency,
    val cadence: String,
    val startsOn: String,
    val endsOn: String? = null,
    val timeZone: String,
    val description: String? = null,
    val accountId: AccountId? = null,
    val categoryId: CategoryId? = null,
)

data class SavingsTermsForm(
    val effectiveOn: String,
    val payoutFrequency: String,
    val capitalize: Boolean?,
    val rateBasis: String,
    val dayCount: String,
    val balanceTiming: String,
    val rounding: String,
)

data class SavingsRateForm(val effectiveOn: String, val nominalAnnualPercent: String)

/** Validates a complete create/edit account form, not a partial repository patch. */
fun validateAccountForm(form: AccountForm): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    try {
        AccountName(form.name)
    } catch (_: IllegalArgumentException) {
        issues.addIssue("name", "invalid_name")
    }

    val hasOpeningAmount = form.openingBalanceAmount != null
    val hasOpeningDate = form.openingBalanceOccurredOn != null
    if (hasOpeningAmount != hasOpeningDate) {
        issues.addIssue("opening_balance", "incomplete_opening_balance")
    } else if (hasOpeningAmount && hasOpeningDate) {
        if (parseMoney(form.openingBalanceAmount, form.currency) == null) {
            issues.addIssue("openingBalanceAmount", "invalid_amount")
        }
        if (parseBusinessDate(form.openingBalanceOccurredOn) == null) {
            issues.addIssue("openingBalanceOccurredOn", "invalid_date")
        }
    }
    return FormValidation(issues)
}

/** Validates the complete merged form; known metadata enables local checks only. */
fun validateTransactionForm(
    form: TransactionForm,
    accounts: List<Account> = emptyList(),
): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    val amount = parseMoney(form.amount, form.currency)
    if (amount == null) {
        issues.addIssue("amount", "invalid_amount")
    } else if (amount.minorUnits <= 0) {
        issues.addIssue("amount", "must_be_positive")
    }
    val date = parseBusinessDate(form.occurredOn)
    if (date == null) issues.addIssue("occurredOn", "invalid_date")
    if (!validDescription(form.description)) issues.addIssue("description", "invalid_description")

    when (form.kind) {
        TransactionKind.TRANSFER -> validateTransfer(form, amount, accounts, issues)
        TransactionKind.INCOME, TransactionKind.EXPENSE, TransactionKind.INTEREST_INCOME -> {
            if (form.destinationAccountId != null || form.receivedAmount != null ||
                form.receivedCurrency != null || form.quotedRate != null
            ) {
                issues.addIssue("transfer", "unexpected_transfer_fields")
            }
            if (form.kind == TransactionKind.INTEREST_INCOME && form.accountId == null) {
                issues.addIssue("accountId", "required_for_interest")
            }
            val account = validateKnownAccount(form.accountId, form.currency, accounts, issues)
            if (form.kind == TransactionKind.INCOME && account?.type == AccountType.SAVINGS) {
                issues.addIssue("accountId", "income_not_allowed_for_savings")
            }
        }
        TransactionKind.OPENING_BALANCE -> issues.addIssue("kind", "unsupported_transaction_kind")
    }

    // Reuse the domain constructor as the final guard for its transaction invariants.
    if (amount != null && date != null && issues.isEmpty()) {
        val received = form.receivedAmount?.let { raw ->
            form.receivedCurrency?.let { currency -> parseMoney(raw, currency) }
        }
        try {
            Transaction(
                id = TransactionId(0, 0),
                kind = form.kind,
                state = TransactionState.POSTED,
                amount = amount,
                occurredOn = date,
                accountId = form.accountId,
                categoryId = form.categoryId,
                description = form.description,
                destinationAccountId = form.destinationAccountId,
                receivedAmount = received,
            )
        } catch (_: IllegalArgumentException) {
            issues.addIssue("form", "transaction_invariant_violation")
        }
    }
    return FormValidation(issues)
}

fun validateRecurringExpenseForm(
    form: RecurringExpenseForm,
    accounts: List<Account> = emptyList(),
): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    val amount = parseMoney(form.amount, form.currency)
    if (amount == null) issues.addIssue("amount", "invalid_amount")
    else if (amount.minorUnits <= 0) issues.addIssue("amount", "must_be_positive")
    if (form.cadence !in setOf("weekly", "monthly", "quarterly", "annual")) {
        issues.addIssue("cadence", "unsupported_cadence")
    }
    val start = parseBusinessDate(form.startsOn)
    if (start == null) issues.addIssue("startsOn", "invalid_date")
    if (form.endsOn != null) {
        val end = parseBusinessDate(form.endsOn)
        if (end == null) issues.addIssue("endsOn", "invalid_date")
        else if (start != null && end < start) issues.addIssue("endsOn", "must_not_precede_start")
    }
    if (!validZoneSyntax(form.timeZone)) issues.addIssue("timeZone", "invalid_zone_syntax")
    if (!validDescription(form.description)) issues.addIssue("description", "invalid_description")
    validateKnownAccount(form.accountId, form.currency, accounts, issues)
    return FormValidation(issues)
}

fun validateSavingsTermsForm(form: SavingsTermsForm): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    if (parseBusinessDate(form.effectiveOn) == null) issues.addIssue("effectiveOn", "invalid_date")
    if (form.payoutFrequency !in setOf("daily", "monthly", "quarterly", "annual")) {
        issues.addIssue("payoutFrequency", "unsupported_payout_frequency")
    }
    if (form.capitalize == null) issues.addIssue("capitalize", "required")
    if (form.rateBasis != "nominal_annual") issues.addIssue("rateBasis", "unsupported_rate_basis")
    if (form.dayCount != "actual_calendar_year") issues.addIssue("dayCount", "unsupported_day_count")
    if (form.balanceTiming != "daily_close") issues.addIssue("balanceTiming", "unsupported_balance_timing")
    if (form.rounding != "period_end_half_even") issues.addIssue("rounding", "unsupported_rounding")
    return FormValidation(issues)
}

fun validateSavingsRateForm(form: SavingsRateForm): FormValidation {
    val issues = mutableListOf<ValidationIssue>()
    if (parseBusinessDate(form.effectiveOn) == null) issues.addIssue("effectiveOn", "invalid_date")
    if (!validSavingsRate(form.nominalAnnualPercent)) {
        issues.addIssue("nominalAnnualPercent", "invalid_rate")
    }
    return FormValidation(issues)
}

private fun validateTransfer(
    form: TransactionForm,
    sent: Money?,
    accounts: List<Account>,
    issues: MutableList<ValidationIssue>,
) {
    if (form.accountId == null) issues.addIssue("accountId", "required_for_transfer")
    if (form.destinationAccountId == null) issues.addIssue("destinationAccountId", "required_for_transfer")
    if (form.accountId != null && form.accountId == form.destinationAccountId) {
        issues.addIssue("destinationAccountId", "accounts_must_be_distinct")
    }
    if (form.categoryId != null) issues.addIssue("categoryId", "not_allowed_for_transfer")

    val received = if (form.receivedAmount == null || form.receivedCurrency == null) {
        issues.addIssue("receivedAmount", "required_for_transfer")
        null
    } else {
        parseMoney(form.receivedAmount, form.receivedCurrency).also {
            if (it == null) issues.addIssue("receivedAmount", "invalid_amount")
            else if (it.minorUnits <= 0) issues.addIssue("receivedAmount", "must_be_positive")
        }
    }
    if (sent != null && received != null && sent.currency == received.currency && sent != received) {
        issues.addIssue("receivedAmount", "same_currency_amounts_must_match")
    }
    if (form.quotedRate != null && !validQuotedRate(form.quotedRate)) {
        issues.addIssue("quotedRate", "invalid_quoted_rate")
    }

    validateKnownAccount(form.accountId, form.currency, accounts, issues)
    if (form.destinationAccountId != null && form.receivedCurrency != null) {
        validateKnownAccount(form.destinationAccountId, form.receivedCurrency, accounts, issues, "destinationAccountId")
    }
}

private fun validateKnownAccount(
    id: AccountId?,
    currency: Currency,
    accounts: List<Account>,
    issues: MutableList<ValidationIssue>,
    field: String = "accountId",
): Account? {
    if (id == null) return null
    val account = accounts.firstOrNull { it.id == id } ?: return null
    if (account.status != AccountStatus.ACTIVE) issues.addIssue(field, "account_not_active")
    if (account.currency != currency) issues.addIssue(field, "currency_mismatch")
    return account
}

private fun parseMoney(value: String, currency: Currency): Money? = try {
    Json.decodeFromJsonElement(MoneyAmountSerializer(currency), JsonPrimitive(value))
} catch (_: SerializationException) {
    null
}

private fun parseBusinessDate(value: String): BusinessDate? {
    if (value.length != 10 || value[4] != '-' || value[7] != '-') return null
    if (value.indices.any { it != 4 && it != 7 && value[it] !in '0'..'9' }) return null
    return try {
        BusinessDate(value.substring(0, 4).toInt(), value.substring(5, 7).toInt(), value.substring(8, 10).toInt())
    } catch (_: IllegalArgumentException) {
        null
    }
}

private fun validDescription(value: String?): Boolean {
    if (value == null) return true
    var scalarCount = 0
    var index = 0
    while (index < value.length) {
        val character = value[index]
        if (character.code in 0..31 || character.code in 127..159) return false
        when (character) {
            in '\uD800'..'\uDBFF' -> {
                if (index + 1 >= value.length || value[index + 1] !in '\uDC00'..'\uDFFF') return false
                index += 2
            }
            in '\uDC00'..'\uDFFF' -> return false
            else -> index++
        }
        scalarCount++
        if (scalarCount > 1000) return false
    }
    return true
}

private fun validZoneSyntax(value: String): Boolean {
    if (value.isEmpty() || value.length > 255) return false
    if (value.any { it.isWhitespace() || it.code in 0..31 || it.code in 127..159 }) return false
    if (value.any { it !in 'A'..'Z' && it !in 'a'..'z' && it !in '0'..'9' && it !in "_+-./" }) return false
    val segments = value.split('/')
    return segments.none { it.isEmpty() || it == "." || it == ".." }
}

private fun validQuotedRate(value: String): Boolean {
    if (value.isEmpty()) return false
    val dot = value.indexOf('.')
    if (dot != value.lastIndexOf('.')) return false
    val integer = if (dot < 0) value else value.substring(0, dot)
    val fraction = if (dot < 0) null else value.substring(dot + 1)
    if (integer.isEmpty() || integer.any { it !in '0'..'9' }) return false
    if (fraction != null && (fraction.isEmpty() || fraction.length > 12 || fraction.any { it !in '0'..'9' })) return false
    return value.any { it in '1'..'9' }
}

private fun validSavingsRate(value: String): Boolean {
    if (value.isEmpty() || value.length > 1024) return false
    val dot = value.indexOf('.')
    if (dot != value.lastIndexOf('.')) return false
    val integer = if (dot < 0) value else value.substring(0, dot)
    val fraction = if (dot < 0) null else value.substring(dot + 1)
    if (integer.isEmpty() || integer.any { it !in '0'..'9' }) return false
    return fraction == null || (fraction.isNotEmpty() && fraction.length <= 6 && fraction.all { it in '0'..'9' })
}

private fun MutableList<ValidationIssue>.addIssue(field: String, code: String) {
    if (none { it.field == field && it.code == code }) add(ValidationIssue(field, code))
}
