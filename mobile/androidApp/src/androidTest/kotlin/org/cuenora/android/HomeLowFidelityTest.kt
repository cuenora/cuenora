package org.cuenora.android

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.planning.*
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

/** BF49A structure, current design-system styling, synthetic financial values only. */
class HomeLowFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val accountCalls = AtomicInteger()
    private val recurrenceCalls = AtomicInteger()
    private val readCalls = AtomicInteger()
    private val data = AccountPlanning("2026-11-16T12:00:00Z", BusinessDate(2026, 11, 30), true, listOf(
        PlanningAccount(AccountId(0, 1), "Account A", AccountType.NORMAL, AccountStatus.ACTIVE,
            Currency.SEK, "300.00", "800.00", "0.00", "-500.00", true),
        PlanningAccount(AccountId(0, 2), "Account B", AccountType.NORMAL, AccountStatus.ACTIVE,
            Currency.SEK, "2000.00", "0.00", "0.00", "2000.00", true),
        PlanningAccount(AccountId(0, 3), "Travel", AccountType.NORMAL, AccountStatus.ACTIVE,
            Currency.EUR, "400.00", "0.00", "0.00", "400.00", true),
        PlanningAccount(AccountId(0, 4), "Savings", AccountType.SAVINGS, AccountStatus.ACTIVE,
            Currency.USD, "120.00", "0.00", "0.00", "120.00", true)),
        listOf(UnassignedPlanning(Currency.EUR, "20.00", "0.00", true)))

    private fun render(width: Int = 390, fontScale: Float = 1f, dark: Boolean = false) {
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> {
                readCalls.incrementAndGet()
                return RepositoryResult.Success(data.copy(through = through ?: data.through))
            }
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, fontScale)) {
                Box(Modifier.width(width.dp).height(844.dp).testTag("home-preview")) {
                    CuenoraTheme(darkTheme = dark) {
                        CuenoraApp(recordActivity = {}, homeContent = {
                            PlanningScreen(session, { action -> scope.launch { action() } },
                                accounts = { accountCalls.incrementAndGet() },
                                recurrences = { recurrenceCalls.incrementAndGet() })
                        })
                    }
                }
            }
        }
    }

    @Test fun lowFidelityOverviewUsesOpenCurrencyRowAndFocusedPlanningCard() {
        render()
        val sek = compose.onNodeWithTag("home-currency-SEK").fetchSemanticsNode().boundsInRoot
        val eur = compose.onNodeWithTag("home-currency-EUR").fetchSemanticsNode().boundsInRoot
        val usd = compose.onNodeWithTag("home-currency-USD").fetchSemanticsNode().boundsInRoot
        assertEquals(sek.top, eur.top, 1f)
        assertEquals(sek.top, usd.top, 1f)
        assertTrue(sek.right < eur.left && eur.right < usd.left)
        compose.onNodeWithText("2300.00").assertExists()
        compose.onNodeWithText("400.00").assertExists()
        compose.onNodeWithText("120.00").assertExists()
        capture("home-light-overview")
        compose.onNodeWithText("View accounts").performClick()
        assertEquals(1, accountCalls.get())
        compose.onNodeWithTag("home-shortfall-indicator-0").assertExists()
        compose.onNodeWithText("SEK -500.00").performScrollTo().assertIsDisplayed()
        capture("home-light-planning")
        compose.onNodeWithTag("home-no-pending-1").assertExists()
        compose.onNodeWithText("Unassigned payment commitments").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("EUR 20.00").assertExists()
        compose.onNodeWithTag("review-planned-payments").performScrollTo().performClick()
        assertEquals(1, recurrenceCalls.get())
        assertEquals("Presentation and navigation must not cause extra reads", 1, readCalls.get())
    }

    @Test fun narrowLargeTextReflowsExactAmountsAndRetainsRefresh() {
        render(width = 320, fontScale = 1.5f, dark = true)
        val summary = compose.onNodeWithTag("planning-balance-summary").fetchSemanticsNode().boundsInRoot
        for (currency in listOf("SEK", "EUR", "USD")) {
            val bounds = compose.onNodeWithTag("home-currency-$currency").fetchSemanticsNode().boundsInRoot
            assertTrue("Currency $currency should stay inside the summary", bounds.left >= summary.left && bounds.right <= summary.right)
        }
        capture("home-dark-large-text")
        compose.onNodeWithText("SEK -500.00").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Estimated shortfall").assertExists()
        compose.onNodeWithText("Refresh planning").performScrollTo().performClick()
        compose.waitForIdle()
        assertEquals(2, readCalls.get())
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use { compose.onNodeWithTag("home-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
    }
}
