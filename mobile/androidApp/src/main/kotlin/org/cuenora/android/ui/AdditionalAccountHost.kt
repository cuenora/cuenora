package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import org.cuenora.shared.onboarding.*

/** Shares the signed-in writer/journal with onboarding; never creates a competing account writer. */
@Composable
fun AdditionalAccountHost(
    flow: OnboardingSession,
    submit: (suspend () -> Unit) -> Unit,
    finished: () -> Unit,
    content: @Composable (() -> Unit, Boolean) -> Unit,
) {
    val state by flow.state.collectAsState()
    var visible by remember(flow) { mutableStateOf(false) }
    Box(Modifier.fillMaxSize()) {
        content({
            if (!flow.state.value.busy) {
                visible = true
                submit { flow.beginAdditional() }
            }
        }, !state.busy)
        if (visible) AdditionalAccountScreen(flow, submit) {
            flow.leaveAdditional()
            visible = false
            finished()
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AdditionalAccountScreen(flow: OnboardingSession, submit: (suspend () -> Unit) -> Unit, done: () -> Unit) {
    val state by flow.state.collectAsState()
    var discard by remember { mutableStateOf(false) }
    val requestBack = {
        if (!state.busy) {
            if (state.stage in setOf(OnboardingStage.Form, OnboardingStage.Review)) discard = true
            else done()
        }
    }
    BackHandler { requestBack() }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = {
        discard = false
        flow.cancel()
        done()
    })
    Scaffold(
        modifier = Modifier.fillMaxSize().testTag("additional-account-screen"),
        containerColor = MaterialTheme.colorScheme.background,
        contentWindowInsets = WindowInsets.safeDrawing,
        topBar = {
            if (state.stage in setOf(OnboardingStage.Form, OnboardingStage.Review)) {
                TextButton(onClick = requestBack, enabled = !state.busy, contentPadding = PaddingValues(0.dp),
                    modifier = Modifier.statusBarsPadding().padding(horizontal = 24.dp).fillMaxWidth()
                        .cuenoraFocus().heightIn(min = 48.dp)
                        .semantics { contentDescription = "Back to accounts" }) {
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = null,
                            modifier = Modifier.size(20.dp).testTag("account-form-back-icon"))
                        Text("Accounts", style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.testTag("account-form-back-label"))
                    }
                }
            } else TopAppBar(title = { Text("Add account") }, navigationIcon = {
                IconButton(onClick = requestBack, enabled = !state.busy) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                }
            }, colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background))
        },
        bottomBar = {
            if (state.stage == OnboardingStage.Complete) Box(Modifier.navigationBarsPadding().padding(16.dp)) {
                CuenoraAction("Done", done, Modifier.testTag("additional-account-done"))
            }
        },
    ) { padding ->
        Box(Modifier.padding(padding).consumeWindowInsets(padding)) {
            OnboardingPanel(state, flow, submit, accountsConnected = true, additional = true, requestCancel = requestBack)
        }
    }
}
