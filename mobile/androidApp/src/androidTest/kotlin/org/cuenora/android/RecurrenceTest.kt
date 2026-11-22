package org.cuenora.android

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.recurrence.*
import org.cuenora.shared.repository.AccountWithBalance
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

    @Test fun upcomingPaymentsShowDueDateAndOpenReviewWithoutPreparingAWrite() {
        render(RecurrenceScreenState(occurrences = listOf(occurrence)))
        compose.onNodeWithText("Upcoming payments").assertExists()
        compose.onNodeWithText("2026-02-04").assertExists()
        compose.onNodeWithText("12.34 SEK").assertExists()
        compose.onNodeWithText("Planned · No account assigned").assertExists()
        compose.onNodeWithText("Record actual payment").assertDoesNotExist()
        click("Review Rent")
        compose.onNodeWithText("Record actual payment").assertExists()
        assertTrue(actions.isEmpty())
    }

    @Test fun ruleManagementOpensSeparatelyAndBackReturnsToPayments() {
        render(RecurrenceScreenState(rules = listOf(rule), occurrences = listOf(occurrence)))
        compose.onNodeWithText("Pause rule").assertDoesNotExist()
        click("Manage recurring rules")
        compose.onNodeWithText("Upcoming payments").assertDoesNotExist()
        compose.onNodeWithText("Pause rule").assertExists()
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Upcoming payments").assertExists()
        assertTrue(actions.isEmpty())
    }

    @Test fun occurrenceDetailShowsPlannedStatusAndRecordedBalanceWithoutPosting() {
        val account = Account(AccountId(0, 4), OwnerId(0, 5), AccountName("Main checking"),
            AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
        render(RecurrenceScreenState(
            occurrences = listOf(occurrence.copy(accountId = account.id)),
            accounts = listOf(AccountWithBalance(account, Money(50000, Currency.SEK))),
        ))
        click("Review Rent")
        compose.onNodeWithText("Planned · Not confirmed as paid").assertExists()
        compose.onNodeWithText("SEK 12.34").assertExists()
        compose.onNodeWithText("Due date").assertExists()
        compose.onNodeWithText("2026-02-04").assertExists()
        compose.onNodeWithText("Main checking · SEK").assertExists()
        compose.onNodeWithText("Recorded balance").assertExists()
        compose.onNodeWithText("SEK 500.00").assertExists()
        compose.onNodeWithText("Cuenora does not send money.", substring = true).assertExists()
        compose.onNodeWithText("Planning difference").assertDoesNotExist()
        click("Not paid yet")
        compose.onNodeWithText("Upcoming payments").assertExists()
        assertTrue(actions.isEmpty())
    }

    @Test fun occurrenceActionsRetainTheExistingReviewBoundary() {
        render(RecurrenceScreenState(occurrences = listOf(occurrence)))
        click("Review Rent")
        compose.onNodeWithText("No account assigned").assertExists()
        compose.onNodeWithText("Recorded balance").assertDoesNotExist()
        click("Change due date")
        compose.onNodeWithText("Postpone one occurrence").assertExists()
        assertTrue(actions.isEmpty())
        click("Cancel changes")
        compose.onNodeWithText("Discard").performClick()
        click("Skip this occurrence")
        assertEquals(listOf(RecurrenceIntent.CancelReview, RecurrenceIntent.Skip(occurrence.id)), actions)
    }

    @Test fun markAsPaidPreparesActualTermsButDoesNotSubmit() {
        render(RecurrenceScreenState(occurrences = listOf(occurrence), profile = RecurrenceProfile(true, "Europe/Stockholm")))
        click("Review Rent")
        click("Record actual payment")
        assertEquals(listOf(RecurrenceIntent.Confirm(occurrence.id,
            ActualPaymentDraft("12.34", Currency.SEK, "2026-02-04", null, null, "Rent"))), actions)
        assertFalse(actions.contains(RecurrenceIntent.Submit))
    }
    @Test fun confirmationShowsActualTermsAndExplicitAccountingEffect() {
        render(RecurrenceScreenState(review = RecurrenceReview.Confirm(occurrence.id,
            ActualPaymentDraft("15.20", Currency.EUR, "2026-11-09"))))
        compose.onNodeWithText("Actual amount: 15.20 EUR").assertExists()
        compose.onNodeWithText("Actual payment date: 2026-11-09").assertExists()
        compose.onNodeWithText("Included in expense reports; no account balance changes.").assertExists()
        assertTrue(actions.isEmpty())
        compose.onNode(hasText("Confirm payment") and hasClickAction()).performScrollTo().performClick()
        assertEquals(listOf(RecurrenceIntent.Submit), actions)
    }
    @Test fun ruleAndOccurrenceActionsRequireReviewAndPreservePlanningCopy() {
        render(RecurrenceScreenState(rules = listOf(rule), occurrences = listOf(occurrence), profile = RecurrenceProfile(true, "UTC")))
        compose.onNodeWithText("Planned payments do not change balances or realized expense reports.").assertExists()
        click("Manage recurring rules")
        click("Pause rule")
        assertEquals(RecurrenceIntent.Pause(rule.id), actions.single())
    }
    @Test fun uncertainOperationBlocksNewActionsAndOffersOnlyOriginalRetry() {
        render(RecurrenceScreenState(pendingRetry = true, rules = listOf(rule), occurrences = listOf(occurrence)))
        compose.onNodeWithText("Add recurring expense").assertDoesNotExist()
        compose.onNodeWithText("Review Rent").assertDoesNotExist()
        compose.onNodeWithText("Manage recurring rules").assertDoesNotExist()
        click("Retry original request")
        assertEquals(listOf(RecurrenceIntent.Retry), actions)
    }
    @Test fun terminalOccurrencesHaveNoPaymentOrSkipControls() {
        render(RecurrenceScreenState(
            occurrences = listOf(occurrence.copy(state = OccurrenceState.SKIPPED)),
            occurrenceFilter = OccurrenceState.SKIPPED,
        ))
        compose.onNodeWithText("Skipped · No account assigned").assertExists()
        compose.onNodeWithText("Review Rent").assertDoesNotExist()
        compose.onNodeWithText("Record actual payment").assertDoesNotExist()
        compose.onNodeWithText("Skip this occurrence").assertDoesNotExist()
    }
    @Test fun pauseReviewExplainsPreservationWithoutPosting() {
        render(RecurrenceScreenState(review = RecurrenceReview.PauseRule(rule.id)))
        compose.onNodeWithText("Existing occurrences remain unchanged and actionable.").assertExists()
        click("Apply planning change")
        assertEquals(listOf(RecurrenceIntent.Submit), actions)
    }

    @Test fun creatingRulePutsExpenseDetailsBeforeScheduleAndKeepsDiscardGuard() {
        render(RecurrenceScreenState(profile = RecurrenceProfile(true, "Europe/Stockholm")))
        click("Add recurring expense")
        compose.onNodeWithText("Add recurring expense").assertExists()
        val fields = listOf(
            compose.onNodeWithTag("recurrence-description"),
            compose.onNodeWithTag("recurrence-rule-amount"),
            compose.onNodeWithText("Currency: SEK"),
            compose.onNodeWithText("Category: No category"),
            compose.onNodeWithText("Source account: No account assigned"),
            compose.onNodeWithText("Repeat: monthly"),
            compose.onNodeWithTag("recurrence-start"),
            compose.onNodeWithTag("recurrence-zone"),
            compose.onNodeWithTag("recurrence-end"),
        )
        assertTrue(fields.map { it.getUnclippedBoundsInRoot().top.value }.zipWithNext().all { (a, b) -> a < b })
        compose.onNodeWithText("Review schedule").assertExists()
        compose.onNodeWithText("Saving a schedule creates planned payments.", substring = true).assertExists()
        click("Cancel")
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Discard").performClick()
        compose.onNodeWithText("Upcoming payments").assertExists()
        assertEquals(listOf(RecurrenceIntent.CancelReview), actions)
    }

    @Test fun editingRuleStillOffersOriginalFieldsAndReviewsAnEdit() {
        render(RecurrenceScreenState(rules = listOf(rule), profile = RecurrenceProfile(true, "UTC")))
        click("Manage recurring rules")
        click("Edit rule")
        compose.onNodeWithText("Edit recurring rule").assertExists()
        compose.onNodeWithText("Cadence: monthly").assertExists()
        compose.onNodeWithText("Account: No account assigned").assertExists()
        compose.onNodeWithText("Cancel changes").assertExists()
        click("Review rule")
        val intent = actions.single() as RecurrenceIntent.Edit
        assertEquals(rule.id, intent.id)
        assertEquals(rule.description, intent.terms.description)
    }

    @Test fun optionalRuleEndDateCanBeClearedAndStartDateStaysCanonical() {
        render(RecurrenceScreenState(profile = RecurrenceProfile(true, "Europe/Stockholm")))
        click("Add recurring expense")
        compose.onNodeWithTag("recurrence-rule-amount").performTextReplacement("12.34")
        compose.selectNativeDate("recurrence-start", 2026, 11, 8)
        compose.selectNativeDate("recurrence-end", 2026, 12, 1)
        compose.clearNativeDate("recurrence-end")
        compose.onNodeWithTag("recurrence-zone").performTextReplacement("Europe/Stockholm")
        click("Review schedule")
        val intent = actions.single() as RecurrenceIntent.Create
        assertEquals("2026-11-08", intent.terms.startsOn)
        assertNull(intent.terms.endsOn)
    }
}
