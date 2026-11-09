package org.cuenora.android

import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.*
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.core.view.WindowInsetsCompat
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import org.cuenora.android.ui.*
import org.cuenora.shared.auth.*
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class IndependentCorrectionChromeTest {
    @get:Rule val compose = createComposeRule()

    @Test fun statusBarPixelsMatchDrawnAppBarInBothThemes() {
        var dark by mutableStateOf(false)
        var expected = 0
        var view: android.view.View? = null
        compose.setContent {
            val activity = LocalActivity.current as ComponentActivity
            SideEffect { activity.enableEdgeToEdge() }
            CuenoraTheme(darkTheme = dark) {
                expected = androidx.compose.material3.MaterialTheme.colorScheme.background.toArgb()
                view = LocalView.current
                CuenoraApp()
            }
        }
        for (mode in listOf(false, true)) {
            compose.runOnIdle { dark = mode }
            compose.waitForIdle()
            val bars = WindowInsetsCompat.toWindowInsetsCompat(view!!.rootWindowInsets)
                .getInsets(WindowInsetsCompat.Type.statusBars()).top
            assertTrue(bars > 0)
            val bitmap = InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
            try { assertEquals(expected, bitmap.getPixel(bitmap.width / 2, bars / 2)) }
            finally { bitmap.recycle() }
        }
    }

    @Test fun renewalKeepsSensitiveSubtreeAndDrawerLogoutClearsExactlyOnce() {
        var now = 0L
        var mounts = 0
        var disposals = 0
        var clears = 0
        var logoutCalls = 0
        var stored: SessionCredentials? = null
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val store = object : SessionStore {
            override suspend fun read() = stored
            override suspend fun replace(credentials: SessionCredentials) { stored = credentials }
            override suspend fun clear() { clears++; stored = null }
        }
        val repository = object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(
                TokenGrant(SessionCredentials("synthetic-a", "synthetic-r"), 60),
            )
            override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> {
                entered.complete(Unit)
                release.await()
                return RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-b", "synthetic-s"), 60))
            }
            override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> {
                logoutCalls++
                return RepositoryResult.Success(Unit)
            }
        }
        val session = AuthSession(repository, store) { now }
        runBlocking { session.login("person@example.test", "x") }
        lateinit var scope: CoroutineScope
        compose.setContent {
            scope = rememberCoroutineScope()
            CuenoraTheme {
                AuthenticationRootWithContent(session, { action -> scope.launch { action() } }) {
                    DisposableEffect(Unit) { mounts++; onDispose { disposals++ } }
                    CuenoraApp()
                }
            }
        }
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.runOnIdle {
            assertEquals(1, mounts)
            assertEquals(0, disposals)
            now = 31_000
            scope.launch { session.renewIfNeeded() }
        }
        compose.waitUntil(5_000) { entered.isCompleted }
        compose.onNodeWithTag("auth-sign-out").assertIsNotEnabled()
        compose.runOnIdle {
            assertEquals(1, mounts)
            assertEquals(0, disposals)
            release.complete(Unit)
        }
        compose.waitUntil(5_000) { session.state.value.phase == AuthPhase.SignedIn }
        compose.onNodeWithTag("auth-sign-out").performClick()
        compose.onNodeWithTag("auth-submit").assertExists()
        compose.runOnIdle {
            assertEquals(1, logoutCalls)
            assertEquals(2, clears) // One renewal cleanup and one local logout clear.
            assertNull(stored)
            assertEquals(1, disposals)
        }
        compose.onNodeWithTag("app-top-bar").assertDoesNotExist()
    }
}
