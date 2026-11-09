package org.cuenora.android

import android.os.Bundle
import android.provider.Settings
import android.view.WindowManager
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.mutableStateOf
import org.cuenora.android.ui.AuthenticationRootWithContent
import org.cuenora.android.ui.OnboardingRoot
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import androidx.compose.runtime.DisposableEffect
import org.cuenora.android.ui.AccountBrowser
import org.cuenora.android.ui.AccountsScreen
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.android.ui.StartupSplashController
import org.cuenora.android.ui.StartupSplashReadiness
import org.cuenora.shared.auth.AuthPhase

class MainActivity : ComponentActivity() {
    private lateinit var startupSplash: StartupSplashController

    override fun onCreate(savedInstanceState: Bundle?) {
        startupSplash = StartupSplashController()
        runCatching {
            installSplashScreen().apply {
                setKeepOnScreenCondition(startupSplash::shouldKeepOnScreen)
                setOnExitAnimationListener { provider ->
                    val scale = runCatching {
                        Settings.Global.getFloat(contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f)
                    }.getOrDefault(1f)
                    startupSplash.onExit(provider, scale, System.currentTimeMillis())
                }
            }
        }.onFailure {
            startupSplash.markReady()
        }
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        val authentication = (application as CuenoraApplication).authentication
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                while (true) {
                    authentication.submit {
                        authentication.session.restore()
                        authentication.session.renewIfNeeded()
                    }
                    delay(1_000)
                }
            }
        }
        enableEdgeToEdge()
        setContent {
            val authState = authentication.session.state.collectAsState().value
            val initialHomeReady = remember { mutableStateOf(false) }
            SideEffect {
                if (StartupSplashReadiness.isReady(authState.phase, null, busy = false)) {
                    startupSplash.markReady()
                } else if ((authState.phase == AuthPhase.SignedIn || authState.phase == AuthPhase.Renewing) && initialHomeReady.value) {
                    startupSplash.markReady()
                }
            }
            CuenoraTheme {
                AuthenticationRootWithContent(authentication.session, authentication::execute) {
                    val onboarding = remember { authentication.newOnboarding() }
                    val accounts = remember { AccountBrowser(authentication.newAccountReader()) }
                    val recording = remember { authentication.newRecording() }
                    DisposableEffect(recording) { onDispose { recording.clear() } }
                    DisposableEffect(accounts) { onDispose { accounts.clear() } }
                    val recordingState = recording.state.collectAsState().value
                    val onboardingState = onboarding.state.collectAsState().value
                    SideEffect {
                        if (StartupSplashReadiness.isReady(authState.phase, onboardingState.stage, onboardingState.busy)) {
                            initialHomeReady.value = true
                            startupSplash.markReady()
                        }
                    }
                    androidx.compose.foundation.layout.Box {
                        OnboardingRoot(onboarding, accountsContent = {
                            AccountsScreen(accounts, submit = authentication::execute, recordActivity = { account ->
                                authentication.execute { recording.begin(account) }
                            })
                        }, submit = authentication::execute, transactionsContent = {
                            org.cuenora.android.ui.RecordingEntry { authentication.execute { recording.begin() } }
                        }, recordActivity = { authentication.execute { recording.begin() } })
                        if (recordingState.stage != org.cuenora.shared.recording.RecordingStage.Closed) {
                            org.cuenora.android.ui.RecordingScreen(recording, authentication::execute) {
                                recording.cancel()
                                authentication.execute { accounts.refresh() }
                            }
                        }
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        if (::startupSplash.isInitialized) startupSplash.close()
        super.onDestroy()
    }
}
