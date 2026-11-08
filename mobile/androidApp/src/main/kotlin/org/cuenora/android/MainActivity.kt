package org.cuenora.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import org.cuenora.android.ui.CuenoraApp
import org.cuenora.android.ui.CuenoraTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            CuenoraTheme { CuenoraApp() }
        }
    }
}
