package org.cuenora.android

import android.os.Bundle
import android.view.WindowManager
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import org.cuenora.android.ui.AuthenticationRootWithContent
import org.cuenora.android.ui.OnboardingRoot
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import androidx.compose.runtime.DisposableEffect
import org.cuenora.android.ui.AccountBrowser
import org.cuenora.android.ui.AccountsScreen
import org.cuenora.android.ui.CuenoraTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
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
                    val recordingState = recording.state.collectAsState().value
                    androidx.compose.foundation.layout.Box {
                        org.cuenora.android.ui.AdditionalAccountHost(onboarding, authentication::execute,
                            finished = { authentication.execute { onboarding.load(); accounts.refresh(); history.refresh() } }) { add, enabled ->
                            OnboardingRoot(onboarding, accountsContent = {
                                AccountsScreen(accounts, addAccount = add, addAccountEnabled = enabled, submit = authentication::execute, recordActivity = { account ->
                                    authentication.execute { recording.begin(account) }
                                })
                            }, submit = authentication::execute, transactionsContent = {
                                org.cuenora.android.ui.TransactionsScreen(history, authentication::execute) {
                                    authentication.execute { recording.begin() }
                                }
                            }, recordActivity = { authentication.execute { recording.begin() } })
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
}
