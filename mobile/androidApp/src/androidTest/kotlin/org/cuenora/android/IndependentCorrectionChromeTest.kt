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

    private data class BarFrame(val appbar: Int, val status: Int)

    /** Wait for the requested content frame, never for a mismatching status bar to become correct. */
    private fun awaitAppbarFrame(expected: Int, capture: () -> BarFrame): BarFrame {
        var last: BarFrame? = null
        repeat(30) {
            val frame = capture()
            if (frame.appbar == expected) return frame
            last = frame
        }
        throw AssertionError("Requested appbar frame did not appear: expected=$expected last=$last")
    }

    private fun assertFrameColors(expected: Int, frame: BarFrame) {
        assertEquals("Appbar must display the requested theme", expected, frame.appbar)
        assertEquals("Status bar must match appbar in the same captured frame", expected, frame.status)
    }

    @Test fun frameOracleSkipsOldContentFrames() {
        val frames = listOf(BarFrame(-1, -1), BarFrame(-16512234, -16512234))
        var reads = 0
        val frame = awaitAppbarFrame(-16512234) { frames[reads++] }
        assertEquals(2, reads)
        assertFrameColors(-16512234, frame)
    }

    @Test fun frameOracleRejectsMismatchingStatusWithoutWaitingItAway() {
        var reads = 0
        val frame = awaitAppbarFrame(-16512234) {
            reads++
            if (reads == 1) BarFrame(-16512234, -1) else BarFrame(-16512234, -16512234)
        }
        assertEquals(1, reads)
        var rejected = false
        try { assertFrameColors(-16512234, frame) } catch (_: AssertionError) { rejected = true }
        assertTrue("A mismatching status bar must fail immediately", rejected)
    }

    @Test fun frameOracleFailsWhenRequestedThemeNeverAppears() {
        var reads = 0
        var rejected = false
        try { awaitAppbarFrame(-16512234) { reads++; BarFrame(-1, -1) } }
        catch (_: AssertionError) { rejected = true }
        assertTrue("An absent target content frame must fail, not skip validation", rejected)
        assertEquals(30, reads)
    }

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
            val appbar = compose.onNodeWithTag("app-top-bar").fetchSemanticsNode().boundsInRoot
            val location = IntArray(2)
            view.getLocationOnScreen(location)
            val frame = awaitAppbarFrame(expected) {
                val bitmap = InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
                try {
                    BarFrame(
                        appbar = bitmap.getPixel(location[0] + appbar.center.x.toInt(),
                            location[1] + appbar.bottom.toInt() - 4),
                        status = bitmap.getPixel(bitmap.width / 2, bars / 2),
                    )
                } finally { bitmap.recycle() }
            }
            assertFrameColors(expected, frame)
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
