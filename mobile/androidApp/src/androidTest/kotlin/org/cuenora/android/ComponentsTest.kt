package org.cuenora.android

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.isFocused
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import org.cuenora.android.ui.CuenoraAmount
import org.cuenora.android.ui.CuenoraAction
import org.cuenora.android.ui.CuenoraApp
import org.cuenora.android.ui.CuenoraDiscardDialog
import org.cuenora.android.ui.CuenoraField
import org.cuenora.android.ui.CuenoraTheme
import androidx.test.espresso.Espresso.pressBack
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class ComponentsTest {
    @get:Rule val compose = createComposeRule()

    @Test fun themeUsesBrandPairsInBothModes() {
        var dark by mutableStateOf(false)
        var background = 0
        var foreground = 0
        compose.setContent {
            CuenoraTheme(dark) {
                background = MaterialTheme.colorScheme.background.toArgb()
                foreground = MaterialTheme.colorScheme.onBackground.toArgb()
            }
        }
        compose.runOnIdle { assertEquals(0xFFFFFFFF.toInt(), background); assertEquals(0xFF0B1E3D.toInt(), foreground) }
        compose.runOnIdle { dark = true }
        compose.runOnIdle { assertEquals(0xFF040B16.toInt(), background); assertEquals(0xFFE9F0FB.toInt(), foreground) }
    }

    @Test fun busyAndUnavailableActionsCannotSubmit() {
        var clicks = 0
        compose.setContent {
            CuenoraTheme {
                Column {
                    CuenoraAction("Continue", { clicks++ })
                    CuenoraAction("Unavailable", { clicks++ }, unavailableReason = "Preview only")
                    CuenoraAction("Busy", { clicks++ }, busy = true)
                }
            }
        }
        compose.onNodeWithText("Continue").assertHeightIsAtLeast(48.dp).performClick()
        compose.onNodeWithText("Unavailable").assertIsNotEnabled().performClick()
        compose.onNodeWithText("Saving…").assertIsNotEnabled().performClick()
        compose.onNodeWithText("Preview only").assertIsDisplayed()
        compose.runOnIdle { assertEquals(1, clicks) }
    }

    @Test fun financialDisplayRetainsExactCurrencyAndSignedAmount() {
        compose.setContent {
            CuenoraTheme { CuenoraAmount("Recorded", "SEK", "+999999999999.99") }
        }
        compose.onNodeWithText("SEK +999999999999.99").assertIsDisplayed()
    }

    @Test fun invalidFieldPreservesEditableInput() {
        compose.setContent {
            CuenoraTheme {
                var value by remember { mutableStateOf("Everyday") }
                CuenoraField("Account name", value, { value = it }, error = "Choose a different name.")
            }
        }
        compose.onNodeWithText("Account name").performTextInput(" test")
        compose.onNodeWithText("Account name").assertTextContains("Everyday", substring = true)
        // Settle the IME viewport before asserting supporting-text visibility.
        pressBack()
        compose.onNodeWithText("Error: Choose a different name.", useUnmergedTree = true).assertIsDisplayed()
    }

    @Test fun discardStartsOnSafeActionAndNeedsExplicitChoice() {
        var discarded = false
        var kept = false
        compose.setContent {
            CuenoraTheme { CuenoraDiscardDialog({ kept = true }, { discarded = true }) }
        }
        // Dialog window focus arrives asynchronously, outside the Compose test clock.
        compose.waitUntil(timeoutMillis = 5_000) {
            compose.onAllNodes(hasText("Keep editing") and isFocused()).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText("Keep editing").assertIsFocused().performClick()
        compose.runOnIdle { assertEquals(true, kept); assertEquals(false, discarded) }
        compose.onNodeWithText("Discard").performClick()
        compose.runOnIdle { assertEquals(true, discarded) }
    }

    private fun renderAt(width: Int, fontScale: Float = 1f) {
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f, fontScale)) {
                Box(Modifier.width(width.dp)) { CuenoraTheme { CuenoraApp() } }
            }
        }
    }

    @Test fun compactWidthUsesNativeBottomBar() {
        renderAt(480)
        compose.onNodeWithTag("navigation-bar").assertIsDisplayed()
        compose.onNodeWithTag("nav-Transactions").performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Transactions")
    }

    @Test fun wideWidthUsesNativeRail() {
        renderAt(720)
        compose.onNodeWithTag("navigation-rail").assertIsDisplayed()
        compose.onNodeWithContentDescription("Open destinations").assertDoesNotExist()
        compose.onNodeWithTag("nav-Accounts").performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Accounts")
    }

    @Test fun backClosesLargeTextDrawerFirst() {
        renderAt(320, 2f)
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("destination-list").assertIsDisplayed()
        pressBack()
        compose.onNodeWithTag("destination-title").assertTextContains("Home").assertIsDisplayed()
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("nav-Accounts").performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Accounts")
    }

    @Test fun largeTextKeepsAllFourDestinationsReachable() {
        renderAt(320, 2f)
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("nav-Transactions").assertIsDisplayed().performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Transactions")
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("nav-Insights").assertIsDisplayed().performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Insights")
    }
}
