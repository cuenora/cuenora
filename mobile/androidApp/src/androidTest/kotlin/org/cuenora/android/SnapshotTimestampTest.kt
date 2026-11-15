package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.planning.*
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Rule
import org.junit.Test

class SnapshotTimestampTest {
    @get:Rule val compose = createComposeRule()

    @Test fun homeDoesNotExposeRawAuditTimestamp() {
        val raw = "2026-11-15T10:22:18.392332Z"
        val data = AccountPlanning(raw, BusinessDate(2026, 11, 30), true, emptyList(), emptyList())
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?) = RepositoryResult.Success(data)
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { PlanningScreen(session, { action -> scope.launch { action() } }) }
        }
        compose.onNodeWithText(raw, substring = true).assertDoesNotExist()
        compose.onNodeWithText("Updated:", substring = true).assertExists()
    }
}
