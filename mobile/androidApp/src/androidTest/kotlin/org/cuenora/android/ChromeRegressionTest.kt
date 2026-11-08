package org.cuenora.android

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.Density
import androidx.core.view.WindowCompat
import org.cuenora.android.ui.LocalSessionNavigationAction
import org.cuenora.android.ui.SessionNavigationAction
import org.cuenora.android.ui.CuenoraApp
import org.cuenora.android.ui.CuenoraTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class ChromeRegressionTest {
    @get:Rule val compose = createComposeRule()

    @Test fun appBarDrawsTheStatusAreaColorInBothThemeModes() {
        var dark by mutableStateOf(false)
        var expectedColor = 0
        var lightStatusIcons: Boolean? = null
        compose.setContent {
            CuenoraTheme(darkTheme = dark) {
                expectedColor = MaterialTheme.colorScheme.background.toArgb()
                val view = LocalView.current
                SideEffect {
                    lightStatusIcons = view.context.findActivity()?.window?.let {
                        WindowCompat.getInsetsController(it, view).isAppearanceLightStatusBars
                    }
                }
                CuenoraApp()
            }
        }
        compose.onNodeWithTag("app-top-bar").assertHeightIsAtLeast(56.dp)
        fun assertAppBarColor() {
            val image = compose.onNodeWithTag("app-top-bar").captureToImage().toPixelMap()
            assertEquals(expectedColor, image[image.width / 2, image.height / 2].toArgb())
        }
        compose.runOnIdle { assertEquals(true, lightStatusIcons) }
        assertAppBarColor()
        compose.runOnIdle { dark = true }
        compose.runOnIdle { assertEquals(false, lightStatusIcons) }
        assertAppBarColor()
    }

    @Test fun signOutIsReachableFromListBottomAndRailNavigationWithoutDuplicateDestinations() {
        var signOutCalls = 0
        var width by mutableStateOf(320.dp)
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                key(width) {
                    Box(Modifier.width(width)) {
                        CuenoraTheme {
                            CompositionLocalProvider(
                                LocalSessionNavigationAction provides SessionNavigationAction(true, "Sign out") { signOutCalls++ },
                            ) { CuenoraApp() }
                        }
                    }
                }
            }
        }
        listOf(320, 480, 720).forEach { widthDp ->
            compose.runOnIdle { width = widthDp.dp }
            compose.onNodeWithContentDescription("Open destinations").performClick()
            compose.onNodeWithTag("auth-sign-out").assertExists()
            assertEquals(1, compose.onAllNodesWithTag("nav-Accounts").fetchSemanticsNodes().size)
            compose.runOnIdle { assertEquals(0, signOutCalls) }
            compose.onNodeWithTag("auth-sign-out").performClick()
            compose.runOnIdle { assertEquals(1, signOutCalls) }
            signOutCalls = 0
        }
    }

    @Test fun renewingSessionDrawerActionIsDisabled() {
        var signOutCalls = 0
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(320.dp)) {
                    CuenoraTheme {
                        CompositionLocalProvider(
                            LocalSessionNavigationAction provides SessionNavigationAction(false, "Renewing session…") {
                                signOutCalls++
                            },
                        ) { CuenoraApp() }
                    }
                }
            }
        }
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("auth-sign-out").assertIsNotEnabled().performClick()
        compose.runOnIdle { assertEquals(0, signOutCalls) }
    }
}

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}
