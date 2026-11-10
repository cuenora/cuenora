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
                    DisposableEffect(onboarding) { onDispose { onboarding.clear() } }
                    val accounts = remember { AccountBrowser(authentication.newAccountReader()) }
                    val history = remember { org.cuenora.android.ui.TransactionBrowser(authentication.newHistoryReader()) }
                    DisposableEffect(history) { onDispose { history.clear() } }
                    val recording = remember { authentication.newRecording() }
                    DisposableEffect(recording) { onDispose { recording.clear() } }
                    DisposableEffect(accounts) { onDispose { accounts.clear() } }
                    val savings = remember { authentication.newSavings() }
                    DisposableEffect(savings) { onDispose { savings.clear() } }
                    val savingsState = savings.state.collectAsState().value
                    val recurrence = remember { authentication.newRecurrence() }
                    val showRecurrence = remember { mutableStateOf(false) }
                    DisposableEffect(recurrence) { onDispose { recurrence.clear() } }
                    val recordingState = recording.state.collectAsState().value
                    val onboardingState = onboarding.state.collectAsState().value
                    SideEffect {
                        if (StartupSplashReadiness.isReady(authState.phase, onboardingState.stage, onboardingState.busy)) {
                            initialHomeReady.value = true
                            startupSplash.markReady()
                        }
                    }
                    androidx.compose.foundation.layout.Box {
                        org.cuenora.android.ui.AdditionalAccountHost(onboarding, authentication::execute,
                            finished = { authentication.execute { onboarding.load(); accounts.refresh(); history.refresh() } }) { add, enabled ->
                            OnboardingRoot(onboarding, accountsContent = {
                                AccountsScreen(accounts, addAccount = add, addAccountEnabled = enabled, savingsActivity = { account ->
                                    authentication.execute { savings.open(account) }
                                }, submit = authentication::execute, recordActivity = { account ->
                                    authentication.execute { recording.begin(account) }
                                })
                            }, submit = authentication::execute, transactionsContent = {
                                org.cuenora.android.ui.TransactionsScreen(history, authentication::execute) {
                                    authentication.execute { recording.begin() }
                                }
                            }, recordActivity = { authentication.execute { recording.begin() } },
                                recurrenceActivity = { showRecurrence.value = true })
                        }
                        if (savingsState.stage != org.cuenora.shared.savings.SavingsStage.Closed) {
                            org.cuenora.android.ui.SavingsScreen(savings, authentication::execute,
                                changed = { authentication.execute { accounts.refresh(); history.refresh() } },
                                done = { savings.close() })
                        }
                        if (showRecurrence.value) {
                            org.cuenora.android.ui.RecurrenceScreen(recurrence, authentication::execute,
                                changed = { authentication.execute { accounts.refresh(); history.refresh() } },
                                done = { showRecurrence.value = false })
                        }
                        if (recordingState.stage != org.cuenora.shared.recording.RecordingStage.Closed) {
                            org.cuenora.android.ui.RecordingScreen(recording, authentication::execute) {
                                recording.cancel()
                                authentication.execute { accounts.refresh(); history.refresh() }
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
