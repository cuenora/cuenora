package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.planning.*
import org.cuenora.shared.recurrence.*
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Rule
import org.junit.Test

class RecurringPaymentNavigationTest {
    @get:Rule val compose = createComposeRule()
    private var navigationCount = 0
    private val occurrence = RecurrenceOccurrence(
        OccurrenceId(0, 3), RuleId(0, 2), BusinessDate(2026, 1, 31),
        BusinessDate(2026, 2, 4), OccurrenceState.PLANNED, Money(1234, Currency.SEK), null, null, "Rent", null,
    )

    private fun renderHome() {
        val data = AccountPlanning("2026-11-10T12:00:00Z", BusinessDate(2026, 11, 30), true, emptyList(), emptyList())
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> = RepositoryResult.Success(data)
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                CuenoraApp(homeContent = {
                    PlanningScreen(session, submit = { action -> scope.launch { action() } }, recurrences = { navigationCount++ })
                })
            }
        }
    }

    @Test fun homeOffersProminentPlannedPaymentReviewRoute() {
        renderHome()

        compose.onNodeWithText("Review planned payments").performScrollTo().assertIsDisplayed().performClick()

        assertEquals(1, navigationCount)
    }

    @Test fun markingPlannedOccurrencePaidOpensActualTermsWithoutSubmitting() {
        val actions = mutableListOf<RecurrenceIntent>()
        compose.setContent {
            CuenoraTheme {
                RecurrencePanel(
                    RecurrenceScreenState(occurrences = listOf(occurrence)),
                    { actions += it },
                    {},
                )
            }
        }

        compose.onNodeWithText("Review Rent").performScrollTo().performClick()
        compose.onNodeWithText("Record actual payment").performScrollTo().performClick()

        compose.onNodeWithTag("recurrence-actual-amount").assertExists()
        compose.onNodeWithTag("recurrence-actual-date").assertExists()
        compose.onNodeWithText("Review actual terms").assertExists()
        assertEquals(listOf(RecurrenceIntent.Confirm(
            occurrence.id,
            ActualPaymentDraft("12.34", Currency.SEK, "2026-02-04", null, null, "Rent"),
        )), actions)
        assertFalse(actions.contains(RecurrenceIntent.Submit))
    }
}
