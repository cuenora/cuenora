package org.cuenora.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import org.cuenora.android.ui.CuenoraApp
import org.cuenora.android.ui.CuenoraTheme

/** Test-only host: never constructs the production client or uses stored credentials. */
class FoundationActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { CuenoraTheme { CuenoraApp() } }
    }
}
