package org.cuenora.android

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.planning.*
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.assertTrue
import java.util.concurrent.atomic.AtomicInteger

/** Financial hierarchy from Pen Home BF49A, using synthetic data only. */
class HomeFidelityTest {
    @get:Rule val compose = createComposeRule()

    @Test fun standardPhoneKeepsPrimaryDestinationsVisible() {
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(390.dp)) {
                    CuenoraTheme { CuenoraApp(homeContent = { Text("Overview") }) }
                }
            }
        }
        compose.onNodeWithTag("navigation-bar").assertIsDisplayed()
        compose.onNodeWithTag("nav-Transactions").assertIsDisplayed().performClick()
        compose.onNodeWithTag("destination-title").assertTextContains("Transactions")
    }

    @Test fun homeRecordActionAppearsBeforeSettings() {
        val recordCalls = AtomicInteger()
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(1f)) {
                Box(Modifier.width(390.dp)) {
                    CuenoraTheme { CuenoraApp(homeContent = { Text("Overview") }, recordActivity = { recordCalls.incrementAndGet() }) }
                }
            }
        }
        val recordNode = compose.onNodeWithContentDescription("Record activity")
        val record = recordNode.fetchSemanticsNode().boundsInRoot.left
        val settings = compose.onNodeWithContentDescription("Settings").fetchSemanticsNode().boundsInRoot.left
        assertTrue("Record activity should appear before Settings", record < settings)
        recordNode.performClick()
        assertTrue("Toolbar action should reuse the existing recording callback", recordCalls.get() == 1)
    }

    @Test fun recordedBalancesLeadAndDateEditingIsProgressivelyDisclosed() {
        val data = AccountPlanning("2026-11-15T12:00:00Z", BusinessDate(2026, 11, 30), true,
            listOf(PlanningAccount(AccountId(0, 1), "Everyday", AccountType.NORMAL, AccountStatus.ACTIVE,
                Currency.SEK, "100.00", "30.00", "0.00", "70.00", true)), emptyList())
        val reader = object : PlanningReader {
            override suspend fun load(through: BusinessDate?) = RepositoryResult.Success(data.copy(through = through ?: data.through))
        }
        val session = PlanningSession(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { PlanningScreen(session, { action -> scope.launch { action() } }) }
        }
        compose.onNodeWithTag("planning-balance-summary").assertExists()
        compose.onAllNodesWithText("SEK 100.00").assertCountEquals(2)
        compose.onNodeWithTag("planning-date").assertDoesNotExist()
        compose.onNodeWithTag("choose-planning-date").performClick()
        compose.onNodeWithTag("planning-date").assertExists()
        compose.onNodeWithTag("planning-date").assert(hasSetTextAction().not())
        compose.onNodeWithText("Use server default").performClick()
        compose.onNodeWithTag("planning-date").assertDoesNotExist()
        compose.onNodeWithText("Through 2026-11-30").assertExists()
    }
}
