package org.cuenora.android

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import org.cuenora.android.ui.CuenoraApp
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.android.ui.LocalSessionNavigationAction
import org.cuenora.android.ui.SessionNavigationAction
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class SettingsNavigationTest {
    @get:Rule val compose = createComposeRule()

    private fun renderWithSessionAction(action: SessionNavigationAction) {
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(480.dp)) {
                    CuenoraTheme {
                        CompositionLocalProvider(LocalSessionNavigationAction provides action) {
                            CuenoraApp()
                        }
                    }
                }
            }
        }
    }

    @Test fun settingsGearReplacesTheNavigationDrawerAndExposesSignOut() {
        var signOutCalls = 0
        renderWithSessionAction(SessionNavigationAction(true, "Sign out") { signOutCalls++ })

        compose.onNodeWithContentDescription("Settings").assertIsDisplayed()
        compose.onNodeWithContentDescription("Open destinations").assertDoesNotExist()
        compose.onNodeWithContentDescription("Settings").performClick()
        compose.onNodeWithText("Settings").assertIsDisplayed()
        compose.onNodeWithTag("auth-sign-out").assertIsDisplayed().performClick()

        compose.runOnIdle { assertEquals(1, signOutCalls) }
    }

    @Test fun settingsCannotSignOutWhileSessionRenewalIsDisabled() {
        var signOutCalls = 0
        renderWithSessionAction(SessionNavigationAction(false, "Renewing session…") { signOutCalls++ })

        compose.onNodeWithContentDescription("Settings").performClick()
        compose.onNodeWithTag("auth-sign-out").assertIsNotEnabled().performClick()

        compose.runOnIdle { assertEquals(0, signOutCalls) }
    }
}
