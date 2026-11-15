package org.cuenora.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag

@Composable
internal fun SettingsPanel(
    action: SessionNavigationAction?,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Settings") },
        confirmButton = {
            if (action != null) {
                TextButton(
                    modifier = Modifier.testTag("auth-sign-out").cuenoraFocus(),
                    enabled = action.enabled,
                    onClick = {
                        if (action.enabled) {
                            onDismiss()
                            action.onSignOut()
                        }
                    },
                ) {
                    Text(action.label)
                }
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, modifier = Modifier.cuenoraFocus()) { Text("Close") }
        },
        modifier = Modifier.testTag("settings-panel"),
    )
}
