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
                    val accounts = remember { AccountBrowser(authentication.newAccountReader()) }
                    DisposableEffect(accounts) { onDispose { accounts.clear() } }
                    OnboardingRoot(onboarding, accountsContent = {
                        AccountsScreen(accounts, authentication::execute)
                    }, submit = authentication::execute)
                }
            }
        }
    }
}
