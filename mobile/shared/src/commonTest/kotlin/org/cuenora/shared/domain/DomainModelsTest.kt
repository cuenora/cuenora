package org.cuenora.shared.domain

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertNull

class DomainModelsTest {
    private val accountId = AccountId(1, 2)
    private val ownerId = OwnerId(3, 4)
    private val date = BusinessDate(2024, 2, 29)

    @Test
    fun recordedMoneyHasExactSymmetricBoundsAndCurrency() {
        for (currency in Currency.entries) {
            for (units in listOf(0L, 1L, -1L, Long.MAX_VALUE, -Long.MAX_VALUE)) {
                assertEquals(units, Money(units, currency).minorUnits)
                assertEquals(currency, Money(units, currency).currency)
            }
            assertFailsWith<IllegalArgumentException> { Money(Long.MIN_VALUE, currency) }
        }
        assertNotEquals(Money(100, Currency.SEK), Money(100, Currency.EUR))
    }

    @Test
    fun accountKeepsIdentityAndMetadataWithoutBalance() {
        for (type in AccountType.entries) {
            for (status in AccountStatus.entries) {
                val account = Account(accountId, ownerId, AccountName("  Savings  "), type, Currency.USD, status)
                assertEquals(accountId, account.id)
                assertEquals(ownerId, account.ownerId)
                assertEquals("Savings", account.name.value)
                assertEquals(type, account.type)
                assertEquals(status, account.status)
                assertEquals(Currency.USD, account.currency)
            }
        }
        assertEquals(AccountName("Savings"), AccountName(" Savings "))
    }

    @Test
    fun accountNamesUseUnicodeScalarsAndRustWhitespacePolicy() {
        assertEquals("My  Savings", AccountName("\u0085My  Savings\u0085").value)
    }

    @Test
    fun accountNamesRejectEmptyControlsAndInvalidUnicode() {
        for (name in listOf(" ", "A\tB", "A\u007fB", "\u001cName", "\ud800", "\udc00")) {
            assertFailsWith<IllegalArgumentException> { AccountName(name) }
        }
        val astral = "\ud83d\udcb0"
        assertEquals(astral.repeat(255), AccountName(astral.repeat(255)).value)
        assertFailsWith<IllegalArgumentException> { AccountName(astral.repeat(256)) }
        assertNotEquals(AccountName("é"), AccountName("e\u0301"))
    }

    @Test
    fun businessDatesAreGregorianAndNotInstants() {
        assertEquals(29, date.day)
        assertEquals(-1, BusinessDate(2023, 12, 31).compareTo(date))
        BusinessDate(2000, 2, 29)
        BusinessDate(1, 1, 1)
        BusinessDate(9999, 12, 31)
        for (parts in listOf(Triple(1900, 2, 29), Triple(2023, 2, 29), Triple(0, 1, 1),
            Triple(10000, 1, 1), Triple(2024, 13, 1), Triple(2024, 4, 31), Triple(2024, 1, 0))) {
            assertFailsWith<IllegalArgumentException> { BusinessDate(parts.first, parts.second, parts.third) }
        }
    }

    @Test
    fun transactionsPreserveAccountlessEntriesAndAllBusinessKinds() {
        for (kind in TransactionKind.entries) {
            for (state in TransactionState.entries) {
                val isTransfer = kind == TransactionKind.TRANSFER
                val accountless = kind == TransactionKind.INCOME || kind == TransactionKind.EXPENSE
                val transaction = Transaction(TransactionId(5, 6), kind, state, Money(123, Currency.SEK), date,
                    accountId = if (accountless) null else accountId,
                    destinationAccountId = if (isTransfer) AccountId(7, 8) else null,
                    receivedAmount = if (isTransfer) Money(123, Currency.SEK) else null)
                assertEquals(kind, transaction.kind)
                assertEquals(state, transaction.state)
                assertEquals(Money(123, Currency.SEK), transaction.amount)
                assertEquals(date, transaction.occurredOn)
                if (accountless) assertNull(transaction.accountId) else assertEquals(accountId, transaction.accountId)
            }
        }
        val transfer = Transaction(TransactionId(5, 6), TransactionKind.TRANSFER, TransactionState.POSTED,
            Money(100, Currency.SEK), date, accountId = accountId, destinationAccountId = AccountId(7, 8),
            receivedAmount = Money(9, Currency.EUR), description = "")
        assertEquals(accountId, transfer.accountId)
        assertEquals(AccountId(7, 8), transfer.destinationAccountId)
        assertEquals(Money(9, Currency.EUR), transfer.receivedAmount)
        assertNull(transfer.categoryId)
        assertEquals("", transfer.description)
    }

    @Test
    fun transactionShapesCannotConfuseTransfersWithIncome() {
        fun event(kind: TransactionKind, source: AccountId? = accountId,
            destination: AccountId? = null, received: Money? = null, amount: Long = 100,
            category: CategoryId? = null) = Transaction(TransactionId(1, 1), kind,
                TransactionState.POSTED, Money(amount, Currency.SEK), date, source, category,
                destinationAccountId = destination, receivedAmount = received)
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.TRANSFER) }
        assertFailsWith<IllegalArgumentException> {
            event(TransactionKind.TRANSFER, destination = accountId, received = Money(100, Currency.SEK))
        }
        assertFailsWith<IllegalArgumentException> {
            event(TransactionKind.TRANSFER, destination = AccountId(7, 8), received = Money(99, Currency.SEK))
        }
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.INCOME, amount = 0) }
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.EXPENSE, amount = -1) }
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.INTEREST_INCOME, source = null) }
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.OPENING_BALANCE, source = null) }
        assertFailsWith<IllegalArgumentException> { event(TransactionKind.INCOME, received = Money(1, Currency.EUR)) }
        val opening = event(TransactionKind.OPENING_BALANCE, amount = -100)
        assertEquals(-100L, opening.amount.minorUnits)
        assertEquals(CategoryId(9, 10), event(TransactionKind.INTEREST_INCOME, category = CategoryId(9, 10)).categoryId)
    }

    @Test
    fun forecastKeepsArbitraryPrecisionAndOpenTailSeparateFromRecordedMoney() {
        val huge = "922337203685477580800000000000000000000"
        val exact = ExactAccrual(huge, "3650000000000000000000000")
        val closed = ForecastPeriod(date, date, exact, ProjectedMinorUnits(huge))
        val open = ForecastPeriod(BusinessDate(2024, 3, 1), BusinessDate(2024, 3, 2), ExactAccrual("0", "1"), null)
        val mutablePeriods = mutableListOf(closed, open)
        val forecast = SavingsForecast(Currency.EUR, mutablePeriods)
        mutablePeriods.clear()
        assertEquals(Currency.EUR, forecast.currency)
        assertEquals(2, forecast.periods.size)
        (forecast.periods as MutableList<ForecastPeriod>).clear()
        assertEquals(2, forecast.periods.size)
        assertEquals(huge, forecast.periods.first().exactMinorUnits.numerator)
        assertEquals(huge, forecast.periods.first().payoutMinorUnits?.digits)
        assertNull(forecast.periods.last().payoutMinorUnits)
        for (invalid in listOf("", "-1", "+1", "1.0", "01", "١")) {
            assertFailsWith<IllegalArgumentException> { ProjectedMinorUnits(invalid) }
            assertFailsWith<IllegalArgumentException> { ExactAccrual(invalid, "1") }
        }
        assertFailsWith<IllegalArgumentException> { ExactAccrual("1", "0") }
        assertFailsWith<IllegalArgumentException> { ExactAccrual("1", "01") }
        assertFailsWith<IllegalArgumentException> { ForecastPeriod(date, BusinessDate(2024, 2, 28), exact, null) }
    }
}
