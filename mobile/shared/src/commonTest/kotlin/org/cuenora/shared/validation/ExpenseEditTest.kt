package org.cuenora.shared.validation

import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import kotlin.test.*

class ExpenseEditTest {
    private val expense = Transaction(TransactionId(0, 1), TransactionKind.EXPENSE,
        TransactionState.POSTED, Money(100, Currency.SEK), BusinessDate(2026, 1, 1),
        accountId = AccountId(0, 2), categoryId = CategoryId(0, 3), description = "Original")

    @Test fun onlyChangedEditableFieldsAreSent() {
        val patch = assertNotNull(expenseEditPatch(expense, "2.50", "2026-01-02", null, "Updated"))
        assertEquals(PatchField.Value(DecimalAmount("2.50")), patch.amount)
        assertEquals(PatchField.Value(BusinessDate(2026, 1, 2)), patch.occurredOn)
        assertEquals(PatchField.Null, patch.categoryId)
        assertEquals(PatchField.Value("Updated"), patch.description)
        assertEquals(PatchField.Absent, patch.accountId)
        assertEquals(PatchField.Absent, patch.currency)
    }

    @Test fun rejectsInvalidAndIneligibleEditsAndOmitsNoOps() {
        assertNull(expenseEditPatch(expense, "1.00", "2026-01-01", expense.categoryId, "Original"))
        assertNull(expenseEditPatch(expense, "0.00", "2026-01-01", null, null))
        assertNull(expenseEditPatch(expense, "2.00", "2026-02-30", null, null))
        assertNull(expenseEditPatch(expense, "2.00", "2026-01-01", null, "Bad\ntext"))
        assertNull(expenseEditPatch(expense.copy(state = TransactionState.VOIDED), "2.00", "2026-01-01", null, null))
    }
}
