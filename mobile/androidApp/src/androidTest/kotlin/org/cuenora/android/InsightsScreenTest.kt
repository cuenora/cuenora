package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.insights.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class InsightsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val range = InsightsDateRange("2024-02-01", "2024-02-29")
    private var opened: Triple<String, String, CategoryId?>? = null

    private fun render() {
        val reader = InsightsReader { requested ->
            val selected = requested ?: range
            RepositoryResult.Success(InsightsReport(selected, listOf(
                InsightsCurrencyReport(Currency.SEK, "12.30", "0.00", "12.30",
                    listOf(InsightsMonth("2024-02", selected.dateFrom, selected.dateTo, "12.30", "0.00", "12.30")),
                    listOf(InsightsCategory(null, null, null, "0.00", null))),
                InsightsCurrencyReport(Currency.EUR, "0.00", "0.00", "0.00", emptyList(), emptyList()),
                InsightsCurrencyReport(Currency.USD, "0.00", "0.00", "0.00", emptyList(), emptyList()),
            )))
        }
        val session = InsightsSession(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                InsightsScreen(session, { action -> scope.launch { action() } }) { from, to, category -> opened = Triple(from, to, category) }
            }
        }
    }

    @Test fun showsRangeCurrencyTotalsZeroStateAndExplicitReportBoundaries() {
        render()
        compose.onNodeWithText("2024-02-01 through 2024-02-29", substring = true).assertExists()
        compose.onNodeWithText("Income · SEK 12.30").assertExists()
        compose.onNodeWithText("Expenses · SEK 0.00").assertExists()
        compose.onNodeWithText("No percentage distribution for zero category spending.").assertExists()
        compose.onNodeWithText("include accountless activity", substring = true).assertExists()
        compose.onNodeWithText("separate by currency", substring = true).assertExists()
    }

    @Test fun uncategorizedLinkIsDateDrilldownOnlyAndExplicitlyBroader() {
        render()
        compose.onNodeWithTag("insights-category-uncategorized").performScrollTo().assertIsDisplayed().performClick()
        compose.runOnIdle { assertEquals(Triple("2024-02-01", "2024-02-29", null), opened) }
        compose.onNodeWithText("Uncategorized opens all categories.", substring = true).assertExists()
    }

    @Test fun initialReadAndRetryBothUseSubmitAndRetryAfterFailureLoadsData() {
        var attempts = 0
        val report = InsightsReport(range, listOf(
            InsightsCurrencyReport(Currency.SEK, "12.30", "0.00", "12.30", emptyList(), emptyList()),
            InsightsCurrencyReport(Currency.EUR, "0.00", "0.00", "0.00", emptyList(), emptyList()),
            InsightsCurrencyReport(Currency.USD, "0.00", "0.00", "0.00", emptyList(), emptyList()),
        ))
        val session = InsightsSession(InsightsReader {
            attempts++
            if (attempts == 1) RepositoryResult.Failure(ApiFailure.Network) else RepositoryResult.Success(report)
        })
        var submissions = 0
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { InsightsScreen(session, { action -> submissions++; scope.launch { action() } }) { _, _, _ -> } }
        }
        compose.onNodeWithText("Error · Insights unavailable").assertExists()
        assertEquals(1, submissions)
        compose.onNodeWithText("Try again").performClick()
        compose.onNodeWithText("Income · SEK 12.30").assertExists()
        assertEquals(2, submissions)
        assertEquals(2, attempts)
    }
}
