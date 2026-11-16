package org.cuenora.android

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.espresso.Espresso.closeSoftKeyboard
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File

/** Synthetic form previews never call a real backend or use personal financial data. */
class AccountFormLowFidelityTest {
    @get:Rule val compose = createComposeRule()
    private var writes = 0
    private var finished = 0

    private fun render(narrow: Boolean = false) {
        val owner = OwnerId(0, 7)
        val backend = object : OnboardingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(true)
            override suspend fun requestVerification() = RepositoryResult.Success(Unit)
            override suspend fun confirmVerification(token: String) = RepositoryResult.Success(Unit)
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
                writes++
                return RepositoryResult.Success(Unit)
            }
        }
        val store = object : PendingAccountStore {
            override suspend fun read(owner: OwnerId): PendingAccount? = null
            override suspend fun save(pending: PendingAccount) = Unit
            override suspend fun clear(owner: OwnerId) = Unit
        }
        val flow = OnboardingSession(backend, store, { IdempotencyKey("synthetic-form-key") }, { "2026-11-16" })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("account-form-preview")) {
                    CuenoraTheme(darkTheme = narrow) {
                        AdditionalAccountHost(flow, { action -> scope.launch { action() } }, { finished++ }) { add, enabled ->
                            CuenoraAction("Add account", add, Modifier.testTag("synthetic-add"), busy = !enabled)
                        }
                    }
                }
            }
        }
        compose.onNodeWithTag("synthetic-add").performClick()
        compose.onNodeWithTag("account-name").assertExists()
    }

    @Test fun formUsesReferenceHierarchyAndSelectorsBeforeExplicitReview() {
        render()
        val back = compose.onNodeWithContentDescription("Back to accounts").assertIsDisplayed().fetchSemanticsNode().boundsInRoot
        val icon = compose.onNodeWithTag("account-form-back-icon", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
        val label = compose.onNodeWithTag("account-form-back-label", useUnmergedTree = true)
            .assertTextEquals("Accounts").fetchSemanticsNode().boundsInRoot
        assertEquals(20f, icon.width, 1f)
        assertEquals(8f, label.left - icon.right, 1f)
        assertTrue(back.height >= 48f)
        val title = compose.onNodeWithTag("account-form-title").assertTextEquals("Add account").assertIsDisplayed().fetchSemanticsNode().boundsInRoot
        val name = compose.onNodeWithTag("account-name").fetchSemanticsNode().boundsInRoot
        val type = compose.onNodeWithTag("account-type-selector").fetchSemanticsNode().boundsInRoot
        val currency = compose.onNodeWithTag("account-currency-selector").fetchSemanticsNode().boundsInRoot
        assertTrue(back.bottom <= title.top && title.bottom <= name.top)
        assertTrue(name.bottom <= type.top && type.bottom <= currency.top)
        capture("account-form-light")
        compose.onNodeWithTag("account-name").performTextReplacement("Travel cash")
        closeSoftKeyboard()
        compose.onNodeWithTag("account-type-selector").performScrollTo().performClick()
        compose.onNodeWithText("Savings").performClick()
        compose.onNodeWithTag("account-currency-selector").performScrollTo().performClick()
        compose.onNodeWithText("USD").performClick()
        compose.onNodeWithTag("account-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your account").assertExists()
        compose.onNodeWithText("Type: Savings").assertExists()
        compose.onNodeWithText("Currency: USD").assertExists()
        compose.onNodeWithText("No opening balance movement.").assertExists()
        compose.runOnIdle { assertEquals(0, writes) }
        compose.onNodeWithContentDescription("Back to accounts").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Discard").performClick()
        compose.runOnIdle { assertEquals(1, finished); assertEquals(0, writes) }
    }

    @Test fun narrowLargeTextKeepsOptionalOpeningAndCancelReachable() {
        render(narrow = true)
        capture("account-form-dark-large-text")
        compose.onNodeWithTag("account-opening-enabled").performScrollTo().performClick()
        compose.onNodeWithTag("account-opening-amount").performScrollTo().performTextReplacement("-12,34")
        closeSoftKeyboard()
        compose.onNodeWithTag("account-opening-amount").assertTextContains("-12.34")
        compose.onNodeWithTag("account-opening-date").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("An opening balance is not income.").performScrollTo().assertIsDisplayed()
        capture("account-form-opening-dark-large-text")
        compose.onNodeWithTag("account-opening-enabled").performScrollTo().performClick()
        compose.onNodeWithTag("account-opening-amount").assertDoesNotExist()
        compose.onNodeWithTag("onboarding-cancel").performScrollTo().performClick()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("account-name").assertExists()
        compose.runOnIdle { assertEquals(0, finished); assertEquals(0, writes) }
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("account-form-preview").captureToImage().asAndroidBitmap()
                .compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
