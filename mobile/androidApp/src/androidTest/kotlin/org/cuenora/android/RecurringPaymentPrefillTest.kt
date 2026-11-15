package org.cuenora.android

import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.text.AnnotatedString
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.recurrence.*
import org.cuenora.shared.repository.AccountWithBalance
import org.cuenora.shared.repository.CatalogCategory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Rule
import org.junit.Test

class RecurringPaymentPrefillTest {
    @get:Rule val compose = createComposeRule()
    private val occurrence = RecurrenceOccurrence(
        OccurrenceId(0, 3), RuleId(0, 2), BusinessDate(2026, 1, 31),
        BusinessDate(2026, 2, 4), OccurrenceState.PLANNED, Money(1234, Currency.SEK), null, null, "Rent", null,
    )
    private val assignedAccountId = AccountId(0, 4)
    private val assignedCategoryId = CategoryId(0, 5)
    private val assignedAccount = Account(
        assignedAccountId, OwnerId(0, 6), AccountName("Main checking"),
        AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE,
    )
    private val assignedCategory = CatalogCategory(assignedCategoryId, "housing", TransactionKind.EXPENSE, "Housing")

    private fun render(state: RecurrenceScreenState, actions: MutableList<RecurrenceIntent>) {
        compose.setContent {
            val current = remember { mutableStateOf(state) }
            CuenoraTheme {
                RecurrencePanel(current.value, { intent ->
                    actions += intent
                    when (intent) {
                        is RecurrenceIntent.Confirm -> current.value = current.value.copy(
                            review = RecurrenceReview.Confirm(intent.id, intent.actual),
                        )
                        RecurrenceIntent.CancelReview -> current.value = current.value.copy(review = null)
                        else -> Unit
                    }
                }, {})
            }
        }
    }

    private fun assertEditableText(tag: String, value: String) {
        compose.onNodeWithTag(tag).assert(
            SemanticsMatcher.expectValue(SemanticsProperties.EditableText, AnnotatedString(value)),
        )
    }

    @Test fun unchangedAccountlessOccurrenceNeedsOnlyExplicitFinalConfirmation() {
        val actions = mutableListOf<RecurrenceIntent>()
        render(RecurrenceScreenState(occurrences = listOf(occurrence)), actions)

        compose.onNodeWithText("Mark as paid").performScrollTo().performClick()

        val expected = RecurrenceIntent.Confirm(
            occurrence.id,
            ActualPaymentDraft("12.34", Currency.SEK, "2026-02-04", null, null, "Rent"),
        )
        assertEquals(listOf(expected), actions)
        assertFalse(actions.contains(RecurrenceIntent.Submit))
        compose.onNodeWithText("Actual amount: 12.34 SEK").assertExists()
        compose.onNodeWithText("Actual payment date: 2026-02-04").assertExists()
        compose.onNodeWithText("Account: No account assigned").assertExists()
        compose.onNodeWithText("Category: No category").assertExists()
        compose.onNodeWithText("Description: Rent").assertExists()
        compose.onNodeWithText("Included in expense reports; no account balance changes.").assertExists()

        compose.onNodeWithText("Confirm actual payment").performScrollTo().performClick()

        assertEquals(listOf(expected, RecurrenceIntent.Submit), actions)
        assertEquals(1, actions.count { it == RecurrenceIntent.Submit })
    }

    @Test fun assignedOccurrenceCanBeEditedThenReviewedBeforeOneExplicitSubmit() {
        val actions = mutableListOf<RecurrenceIntent>()
        val assignedOccurrence = occurrence.copy(
            accountId = assignedAccountId,
            categoryId = assignedCategoryId,
            description = "Utilities",
        )
        val state = RecurrenceScreenState(
            occurrences = listOf(assignedOccurrence),
            accounts = listOf(AccountWithBalance(assignedAccount, Money(50000, Currency.SEK))),
            categories = listOf(assignedCategory),
            profile = RecurrenceProfile(true, "Europe/Stockholm"),
        )
        render(state, actions)

        compose.onNodeWithText("Mark as paid").performScrollTo().performClick()

        val initial = RecurrenceIntent.Confirm(
            assignedOccurrence.id,
            ActualPaymentDraft("12.34", Currency.SEK, "2026-02-04", assignedAccountId, assignedCategoryId, "Utilities"),
        )
        assertEquals(listOf(initial), actions)
        compose.onNodeWithText("Account: Main checking").assertExists()
        compose.onNodeWithText("Category: Housing").assertExists()
        compose.onNodeWithText("Description: Utilities").assertExists()
        assertFalse(actions.contains(RecurrenceIntent.Submit))

        compose.onNodeWithText("Keep editing").performScrollTo().performClick()
        assertEditableText("recurrence-actual-amount", "12.34")
        compose.onNodeWithTag("recurrence-actual-date").assertTextContains("2026-02-04")
        assertEditableText("recurrence-actual-description", "Utilities")
        compose.onNodeWithTag("recurrence-actual-amount").performTextReplacement("15.20")
        compose.selectNativeDate("recurrence-actual-date", 2026, 2, 3)
        compose.onNodeWithTag("recurrence-actual-description").performTextReplacement("Water bill")
        compose.onNodeWithText("Review actual terms").performScrollTo().performClick()

        val edited = RecurrenceIntent.Confirm(
            assignedOccurrence.id,
            ActualPaymentDraft("15.20", Currency.SEK, "2026-02-03", assignedAccountId, assignedCategoryId, "Water bill"),
        )
        assertEquals(listOf(initial, RecurrenceIntent.CancelReview, edited), actions)
        assertFalse(actions.contains(RecurrenceIntent.Submit))
        compose.onNodeWithText("Actual amount: 15.20 SEK").assertExists()
        compose.onNodeWithText("Actual payment date: 2026-02-03").assertExists()
        compose.onNodeWithText("Description: Water bill").assertExists()

        compose.onNodeWithText("Confirm actual payment").performScrollTo().performClick()

        assertEquals(listOf(initial, RecurrenceIntent.CancelReview, edited, RecurrenceIntent.Submit), actions)
        assertEquals(1, actions.count { it == RecurrenceIntent.Submit })
    }
}
