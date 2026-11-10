package org.cuenora.android

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import org.cuenora.android.ui.*
import org.junit.Rule
import org.junit.Test

class DashboardNavigationTest {
    @get:Rule val compose = createComposeRule()

    @Test fun homeInsightsAndHistoryUseConnectedSlotsAndReturnWithoutReplacingTheFoundation() {
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(480.dp)) {
                    CuenoraTheme {
                        CuenoraApp(
                            accountsContent = { Text("Connected accounts") },
                            transactionsContent = { Text("Connected history") },
                            insightsContent = {
                                val navigate = LocalDestinationNavigation.current
                                TextButton(onClick = { navigate(Destination.Transactions) }) {
                                    Text("Report history")
                                }
                            },
                            homeContent = {
                                val navigate = LocalDestinationNavigation.current
                                TextButton(onClick = { navigate(Destination.Accounts) }) {
                                    Text("Planning accounts")
                                }
                            },
                        )
                    }
                }
            }
        }
        compose.onNodeWithText("Planning accounts").performClick()
        compose.onNodeWithText("Connected accounts").assertExists()
        compose.onNodeWithTag("nav-Insights").performClick()
        compose.onNodeWithText("Report history").performClick()
        compose.onNodeWithText("Connected history").assertExists()
        compose.onNodeWithTag("nav-Home").performClick()
        compose.onNodeWithText("Planning accounts").assertExists()
        compose.onNodeWithText("Android foundation").assertDoesNotExist()
    }

    @Test fun absentFinancialSlotsKeepTheExistingFoundationNavigation() {
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(480.dp)) { CuenoraTheme { CuenoraApp() } }
            }
        }
        compose.onNodeWithText("Made for Android").assertExists()
        compose.onNodeWithTag("nav-Insights").performClick()
        compose.onNodeWithText("Insights will be connected in a later roadmap task.").assertExists()
    }
}
