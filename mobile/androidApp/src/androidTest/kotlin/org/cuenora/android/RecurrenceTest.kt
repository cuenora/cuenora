package org.cuenora.android

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.recurrence.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class RecurrenceTest {
    @get:Rule val compose = createComposeRule()
    private val occurrence = RecurrenceOccurrence(OccurrenceId(0, 3), RuleId(0, 2), BusinessDate(2026, 1, 31),
        BusinessDate(2026, 2, 4), OccurrenceState.PLANNED, Money(1234, Currency.SEK), null, null, "Rent", null)
    private val rule = RecurrenceRule(RuleId(0, 2), Money(1234, Currency.SEK), "monthly", BusinessDate(2026, 1, 31),
        null, "Europe/Stockholm", null, null, "Rent", RuleState.ACTIVE, null, null)
    private var actions = mutableListOf<RecurrenceIntent>()
    private fun render(state: RecurrenceScreenState) {
        compose.setContent { CuenoraTheme { RecurrencePanel(state, { actions += it }, {}) } }
    }
    private fun click(text: String) = compose.onNodeWithText(text).performScrollTo().performClick()

    @Test fun plannedReviewDoesNotAutomaticallyPostAndNotPaidReturnsWithoutConfirmation() {
        render(RecurrenceScreenState(occurrences = listOf(occurrence), profile = RecurrenceProfile(true, "Europe/Stockholm")))
        click("Review actual payment")
        compose.onNodeWithTag("recurrence-actual-amount").assertExists()
        assertTrue(actions.isEmpty())
        click("Not paid yet")
        compose.onNodeWithText("Review actual payment").assertExists()
        assertTrue(actions.isEmpty())
    }
    @Test fun confirmationShowsActualTermsAndExplicitAccountingEffect() {
        render(RecurrenceScreenState(review = RecurrenceReview.Confirm(occurrence.id,
            ActualPaymentDraft("15.20", Currency.EUR, "2026-11-09"))))
        compose.onNodeWithText("Actual amount: 15.20 EUR").assertExists()
        compose.onNodeWithText("Actual payment date: 2026-11-09").assertExists()
        compose.onNodeWithText("Included in expense reports; no account balance changes.").assertExists()
        assertTrue(actions.isEmpty())
        click("Confirm actual payment")
        assertEquals(listOf(RecurrenceIntent.Submit), actions)
    }
    @Test fun ruleAndOccurrenceActionsRequireReviewAndPreservePlanningCopy() {
        render(RecurrenceScreenState(rules = listOf(rule), occurrences = listOf(occurrence), profile = RecurrenceProfile(true, "UTC")))
        click("Pause rule")
        assertEquals(RecurrenceIntent.Pause(rule.id), actions.single())
        compose.onNodeWithText("Planned payments do not change balances or realized expense reports.").assertExists()
    }
    @Test fun uncertainOperationBlocksNewActionsAndOffersOnlyOriginalRetry() {
        render(RecurrenceScreenState(pendingRetry = true, rules = listOf(rule), occurrences = listOf(occurrence)))
        compose.onNodeWithText("Create recurring expense").assertDoesNotExist()
        compose.onNodeWithText("Review actual payment").assertDoesNotExist()
        click("Retry original request")
        assertEquals(listOf(RecurrenceIntent.Retry), actions)
    }
    @Test fun terminalOccurrencesHaveNoPaymentOrSkipControls() {
        render(RecurrenceScreenState(occurrences = listOf(occurrence.copy(state = OccurrenceState.SKIPPED))))
        compose.onNodeWithText("Review actual payment").assertDoesNotExist()
        compose.onNodeWithText("Skip occurrence").assertDoesNotExist()
    }
    @Test fun pauseReviewExplainsPreservationWithoutPosting() {
        render(RecurrenceScreenState(review = RecurrenceReview.PauseRule(rule.id)))
        compose.onNodeWithText("Existing occurrences remain unchanged and actionable.").assertExists()
        click("Apply planning change")
        assertEquals(listOf(RecurrenceIntent.Submit), actions)
    }
}
