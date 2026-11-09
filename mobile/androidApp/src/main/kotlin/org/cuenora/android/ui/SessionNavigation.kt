package org.cuenora.android.ui

import androidx.compose.runtime.compositionLocalOf

internal data class SessionNavigationAction(
    val enabled: Boolean,
    val label: String,
    val onSignOut: () -> Unit,
)

internal val LocalSessionNavigationAction = compositionLocalOf<SessionNavigationAction?> { null }
