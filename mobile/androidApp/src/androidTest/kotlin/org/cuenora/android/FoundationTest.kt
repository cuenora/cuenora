package org.cuenora.android

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithContentDescription
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import org.junit.Rule
import org.junit.Test

class FoundationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun appStartsAtHome() {
        compose.onNodeWithTag("destination-title").assertTextEquals("Home").assertIsDisplayed()
    }

    private fun openDestinations() {
        if (compose.onAllNodesWithContentDescription("Open destinations").fetchSemanticsNodes().isNotEmpty()) {
            compose.onNodeWithContentDescription("Open destinations").performClick()
        }
    }

    @Test fun allDestinationsNavigateAndBackReturnsHome() {
        for (destination in listOf("Accounts", "Transactions", "Insights", "Home")) {
            openDestinations()
            compose.onNodeWithTag("nav-$destination").performClick()
            compose.onNodeWithTag("destination-title").assertTextEquals(destination)
        }
        openDestinations()
        compose.onNodeWithTag("nav-Accounts").performClick()
        openDestinations()
        compose.onNodeWithTag("nav-Transactions").performClick()
        compose.activityRule.scenario.onActivity { it.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("destination-title").assertTextEquals("Home")
    }

    @Test fun recreationRestoresDestination() {
        openDestinations()
        compose.onNodeWithTag("nav-Insights").performClick()
        compose.activityRule.scenario.recreate()
        compose.onNodeWithTag("destination-title").assertTextEquals("Insights")
    }

    @Test fun reselectingDestinationDoesNotDuplicateBackStack() {
        repeat(3) {
            openDestinations()
            compose.onNodeWithTag("nav-Accounts").performClick()
        }
        compose.activityRule.scenario.onActivity { it.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("destination-title").assertTextEquals("Home")
    }
}
