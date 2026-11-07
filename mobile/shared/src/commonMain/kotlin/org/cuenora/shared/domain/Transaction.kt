package org.cuenora.shared.domain

enum class TransactionKind { INCOME, EXPENSE, TRANSFER, OPENING_BALANCE, INTEREST_INCOME }
enum class TransactionState { POSTED, VOIDED }

/** One recorded event, not a posting command or an account-balance calculator. */
data class Transaction(
    val id: TransactionId,
    val kind: TransactionKind,
    val state: TransactionState,
    val amount: Money,
    val occurredOn: BusinessDate,
    val accountId: AccountId? = null,
    val categoryId: CategoryId? = null,
    val description: String? = null,
    val destinationAccountId: AccountId? = null,
    val receivedAmount: Money? = null,
) {
    init {
        if (kind == TransactionKind.TRANSFER) {
            require(accountId != null && destinationAccountId != null && accountId != destinationAccountId) {
                "Transfer requires two distinct accounts"
            }
            require(amount.minorUnits > 0 && receivedAmount != null && receivedAmount.minorUnits > 0) {
                "Transfer amounts must be positive"
            }
            require(categoryId == null) { "Transfer cannot have an income or expense category" }
            require(amount.currency != receivedAmount.currency || amount == receivedAmount) {
                "Same-currency transfer amounts must match"
            }
        } else {
            require(destinationAccountId == null && receivedAmount == null) { "Only transfers have a received amount" }
            if (kind != TransactionKind.OPENING_BALANCE) {
                require(amount.minorUnits > 0) { "Income, expense and interest amounts must be positive" }
            }
            if (kind == TransactionKind.OPENING_BALANCE || kind == TransactionKind.INTEREST_INCOME) {
                require(accountId != null) { "Opening and interest require an account" }
            }
            if (kind == TransactionKind.OPENING_BALANCE) {
                require(categoryId == null) { "Opening cannot have an income or expense category" }
            }
        }
    }
}
