package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.planning.*
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class PlanningScreenTest {
    @get:Rule val compose = createComposeRule()
    private var failure = false
    private val cutoff = BusinessDate(2026, 11, 30)
    private val data = AccountPlanning("2026-11-10T12:00:00Z", cutoff, true, listOf(
        PlanningAccount(AccountId(0, 1), "Archived savings", AccountType.SAVINGS, AccountStatus.ARCHIVED, Currency.SEK,
            "300.00", "200.00", "600.00", "-500.00", true)), listOf(
        UnassignedPlanning(Currency.SEK,"20.00","30.00",true), UnassignedPlanning(Currency.EUR,"0.00","0.00",true), UnassignedPlanning(Currency.USD,"0.00","0.00",true)))

    private fun render() {
        val reader = object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> = if (failure) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(data.copy(through = through ?: cutoff))
        }
        val session = PlanningSession(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { CuenoraApp(homeContent = { PlanningScreen(session, submit = { action -> scope.launch { action() } }) }) }
        }
    }

    @Test fun displaysExactSeparatedAccountPlanningAndUnassignedAmounts() {
        render()
        compose.onNodeWithText("Archived savings").assertExists()
        compose.onNodeWithText("Savings · Archived account · SEK").assertExists()
        compose.onNodeWithText("Estimated shortfall").assertExists()
        compose.onNodeWithText("SEK -500.00").assertExists()
        compose.onNodeWithText("SEK pending").assertExists()
        compose.onNodeWithText("SEK 20.00").assertExists()
        compose.onNodeWithText("Updated:", substring = true).assertExists()
        compose.onNodeWithText("Through 2026-11-30").assertExists()
    }

    @Test fun failedReadDoesNotShowZeroOrEmptyAccounts() {
        failure = true; render()
        compose.onNodeWithText("Error · Account planning is unavailable").assertExists()
        compose.onNodeWithText("No accounts yet").assertDoesNotExist()
        compose.onNodeWithText("SEK 0.00").assertDoesNotExist()
    }

    @Test fun explicitDateApplyAndServerDefaultResetRetainUserChoice() {
        render()
        compose.onNodeWithTag("choose-planning-date").performScrollTo().performClick()
        compose.onNodeWithTag("planning-date").performTextReplacement("2027-02-28")
        compose.onNodeWithTag("apply-planning-date").performClick()
        compose.onNodeWithText("Through 2027-02-28").assertExists()
        compose.onNodeWithTag("choose-planning-date").performScrollTo().performClick()
        compose.onNodeWithText("Use server default").performClick()
        compose.onNodeWithText("Updated:", substring = true).assertExists()
        compose.onNodeWithText("Through 2026-11-30").assertExists()
    }
}
