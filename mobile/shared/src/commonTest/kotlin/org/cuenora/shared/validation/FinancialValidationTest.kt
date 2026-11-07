package org.cuenora.shared.validation

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.cuenora.shared.domain.Account
import org.cuenora.shared.domain.AccountId
import org.cuenora.shared.domain.AccountName
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.domain.TransactionKind

class FinancialValidationTest {
    @Test
    fun accountFormUsesDomainNameAndCompleteOpeningTerms() {
        assertTrue(validateAccountForm(AccountForm("  Emergency fund  ", AccountType.SAVINGS, Currency.SEK)).isLocallyValid)
        assertTrue(
            validateAccountForm(
                AccountForm("Savings", AccountType.SAVINGS, Currency.SEK, "0.00", "2024-02-29"),
            ).isLocallyValid,
        )
        assertFalse(validateAccountForm(AccountForm("\t", AccountType.NORMAL, Currency.EUR)).isLocallyValid)
        assertFalse(validateAccountForm(AccountForm("x".repeat(256), AccountType.NORMAL, Currency.EUR)).isLocallyValid)
        assertFalse(validateAccountForm(AccountForm("Savings", AccountType.SAVINGS, Currency.SEK, "1.00", null)).isLocallyValid)
        assertFalse(validateAccountForm(AccountForm("Savings", AccountType.SAVINGS, Currency.SEK, "1.001", "2024-02-29")).isLocallyValid)
        assertFalse(validateAccountForm(AccountForm("Savings", AccountType.SAVINGS, Currency.SEK, "1.00", "1900-02-29")).isLocallyValid)
    }

    @Test
    fun ordinaryTransactionsAllowAccountlessEntriesAndRequireExactPositiveMoney() {
        val accountlessIncome = transaction(kind = TransactionKind.INCOME, accountId = null)
        assertTrue(accountlessIncome.isLocallyValid)
        assertTrue(accountlessIncome.serverChecksRequired)
        assertTrue(transaction(kind = TransactionKind.EXPENSE, accountId = null).isLocallyValid)
        assertFalse(transaction(amount = "0.00").isLocallyValid)
        assertFalse(transaction(amount = "-0.01").isLocallyValid)
        assertFalse(transaction(amount = "1.001").isLocallyValid)
        assertFalse(transaction(occurredOn = "2025-02-29").isLocallyValid)
        assertTrue(transaction(occurredOn = "2000-02-29").isLocallyValid)
        assertFalse(transaction(description = "a".repeat(1001)).isLocallyValid)
        assertTrue(transaction(description = "😀".repeat(1000)).isLocallyValid)
    }

    @Test
    fun localAccountMetadataChecksStatusCurrencyAndSavingsIncomeWithoutAuthorityClaims() {
        val savings = account(AccountType.SAVINGS, Currency.SEK)
        val normal = account(AccountType.NORMAL, Currency.EUR, id = AccountId(3, 4))
        val archived = account(AccountType.NORMAL, Currency.SEK, AccountStatus.ARCHIVED, AccountId(5, 6))

        assertFalse(transaction(kind = TransactionKind.INCOME, accountId = savings.id, accounts = listOf(savings)).isLocallyValid)
        assertTrue(transaction(kind = TransactionKind.INTEREST_INCOME, accountId = savings.id, accounts = listOf(savings)).isLocallyValid)
        assertTrue(transaction(accountId = savings.id, accounts = listOf(savings)).isLocallyValid)
        assertFalse(transaction(accountId = normal.id, accounts = listOf(normal)).isLocallyValid)
        assertFalse(transaction(accountId = archived.id, accounts = listOf(archived)).isLocallyValid)
        assertTrue(transaction(accountId = AccountId(99, 100), accounts = listOf(normal)).isLocallyValid)
    }

    @Test
    fun transferChecksBothSidesAndExactQuoteSyntax() {
        assertTrue(transfer(quotedRate = null).isLocallyValid)
        assertTrue(transfer(quotedRate = "1.250000000001").isLocallyValid)
        assertFalse(transfer(destinationAccountId = AccountId(1, 2)).isLocallyValid)
        assertFalse(transfer(amount = "0.00").isLocallyValid)
        assertFalse(transfer(receivedAmount = "-1.00").isLocallyValid)
        assertFalse(transfer(receivedCurrency = Currency.SEK).isLocallyValid)
        assertFalse(transfer(quotedRate = "1e2").isLocallyValid)
        assertFalse(transfer(quotedRate = "0").isLocallyValid)
        assertFalse(transfer(quotedRate = "1." + "2".repeat(13)).isLocallyValid)

        val source = account(AccountType.NORMAL, Currency.SEK)
        val destination = account(AccountType.NORMAL, Currency.EUR, id = AccountId(3, 4))
        assertTrue(transfer(sourceAccountId = source.id, destinationAccountId = destination.id, accounts = listOf(source, destination)).isLocallyValid)
        assertFalse(transfer(sourceAccountId = source.id, destinationAccountId = source.id, accounts = listOf(source)).isLocallyValid)
        assertTrue(transfer(accounts = listOf(source, destination)).isLocallyValid)
    }

    @Test
    fun recurringExpenseTermsValidateCadenceDatesZoneSyntaxAndDescription() {
        assertTrue(validateRecurringExpenseForm(recurring()).isLocallyValid)
        assertFalse(validateRecurringExpenseForm(recurring(cadence = "biweekly")).isLocallyValid)
        assertFalse(validateRecurringExpenseForm(recurring(startsOn = "2023-02-29")).isLocallyValid)
        assertFalse(validateRecurringExpenseForm(recurring(startsOn = "2024-03-01", endsOn = "2024-02-29")).isLocallyValid)
        assertFalse(validateRecurringExpenseForm(recurring(timeZone = "Europe//Stockholm")).isLocallyValid)
        assertTrue(validateRecurringExpenseForm(recurring(timeZone = "Mars/Base")).isLocallyValid)
        assertFalse(validateRecurringExpenseForm(recurring(description = "x".repeat(1001))).isLocallyValid)
        assertTrue(validateRecurringExpenseForm(recurring(description = "😀".repeat(1000))).isLocallyValid)
    }

    @Test
    fun savingsTermsRequireExplicitCapitalizationAndFixedInterestPolicies() {
        assertTrue(validateSavingsTermsForm(savingsTerms()).isLocallyValid)
        assertFalse(validateSavingsTermsForm(savingsTerms(payoutFrequency = "weekly")).isLocallyValid)
        assertFalse(validateSavingsTermsForm(savingsTerms(capitalize = null)).isLocallyValid)
        assertFalse(validateSavingsTermsForm(savingsTerms(rateBasis = "effective_annual")).isLocallyValid)
        assertFalse(validateSavingsTermsForm(savingsTerms(effectiveOn = "2024-02-30")).isLocallyValid)
    }

    @Test
    fun savingsRateUsesBoundedUnsignedExactGrammarAndAllowsZero() {
        assertTrue(validateSavingsRateForm(SavingsRateForm("2024-01-01", "0")).isLocallyValid)
        assertTrue(validateSavingsRateForm(SavingsRateForm("2024-01-01", "0002.123456")).isLocallyValid)
        assertFalse(validateSavingsRateForm(SavingsRateForm("2024-01-01", "-1")).isLocallyValid)
        assertFalse(validateSavingsRateForm(SavingsRateForm("2024-01-01", "+1")).isLocallyValid)
        assertFalse(validateSavingsRateForm(SavingsRateForm("2024-01-01", "1.1234567")).isLocallyValid)
        assertFalse(validateSavingsRateForm(SavingsRateForm("2024-01-01", "1." + "1".repeat(1023))).isLocallyValid)
        assertFalse(validateSavingsRateForm(SavingsRateForm("2024-02-30", "1")).isLocallyValid)
    }

    private fun transaction(
        kind: TransactionKind = TransactionKind.EXPENSE,
        amount: String = "1.00",
        occurredOn: String = "2024-02-29",
        accountId: AccountId? = null,
        description: String? = null,
        accounts: List<Account> = emptyList(),
    ): FormValidation = validateTransactionForm(
        TransactionForm(kind, amount, Currency.SEK, occurredOn, accountId = accountId, description = description),
        accounts,
    )

    private fun transfer(
        amount: String = "10.00",
        receivedAmount: String = "9.00",
        receivedCurrency: Currency = Currency.EUR,
        sourceAccountId: AccountId? = AccountId(1, 2),
        destinationAccountId: AccountId? = AccountId(3, 4),
        quotedRate: String? = "0.9",
        accounts: List<Account> = emptyList(),
    ): FormValidation = validateTransactionForm(
        TransactionForm(
            TransactionKind.TRANSFER,
            amount,
            Currency.SEK,
            "2024-02-29",
            accountId = sourceAccountId,
            destinationAccountId = destinationAccountId,
            receivedAmount = receivedAmount,
            receivedCurrency = receivedCurrency,
            quotedRate = quotedRate,
        ),
        accounts,
    )

    private fun recurring(
        cadence: String = "monthly",
        startsOn: String = "2024-01-31",
        endsOn: String? = "2025-01-31",
        timeZone: String = "Europe/Stockholm",
        description: String? = "Rent",
    ) = RecurringExpenseForm(
        amount = "500.00",
        currency = Currency.SEK,
        cadence = cadence,
        startsOn = startsOn,
        endsOn = endsOn,
        timeZone = timeZone,
        description = description,
    )

    private fun savingsTerms(
        effectiveOn: String = "2024-01-01",
        payoutFrequency: String = "monthly",
        capitalize: Boolean? = false,
        rateBasis: String = "nominal_annual",
    ) = SavingsTermsForm(
        effectiveOn = effectiveOn,
        payoutFrequency = payoutFrequency,
        capitalize = capitalize,
        rateBasis = rateBasis,
        dayCount = "actual_calendar_year",
        balanceTiming = "daily_close",
        rounding = "period_end_half_even",
    )

    private fun account(
        type: AccountType,
        currency: Currency,
        status: AccountStatus = AccountStatus.ACTIVE,
        id: AccountId = AccountId(1, 2),
    ) = Account(id, OwnerId(7, 8), AccountName("Account"), type, currency, status)
}
