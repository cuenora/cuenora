package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.DialogWindowProvider
import androidx.compose.ui.window.SecureFlagPolicy
import androidx.core.view.WindowCompat
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.TransactionKind
import org.cuenora.shared.repository.TransactionAccountFilter

@Composable
internal fun TransactionFiltersDialog(browser: TransactionBrowser, submit: (suspend () -> Unit) -> Unit) {
    var visiblePopups by remember { mutableIntStateOf(0) }
    Dialog(onDismissRequest = { if (visiblePopups == 0) browser.cancelFilters() },
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false,
            dismissOnBackPress = visiblePopups == 0, securePolicy = SecureFlagPolicy.Inherit)) {
        val view = LocalView.current
        val window = (view.parent as? DialogWindowProvider)?.window
        val lightBackground = MaterialTheme.colorScheme.background.luminance() > 0.5f
        SideEffect {
            window?.let {
                WindowCompat.getInsetsController(it, view).apply {
                    isAppearanceLightStatusBars = lightBackground
                    isAppearanceLightNavigationBars = lightBackground
                }
            }
        }
        TransactionFiltersScreen(browser, submit) { visible -> visiblePopups += if (visible) 1 else -1 }
    }
}

/** Only presentation changes here: the browser owns drafts, validation and server queries. */
@Composable
internal fun TransactionFiltersScreen(browser: TransactionBrowser, submit: (suspend () -> Unit) -> Unit,
    onPopupVisibilityChange: (Boolean) -> Unit = {}) {
    val state by browser.state.collectAsState()
    Scaffold(modifier = Modifier.fillMaxSize().testTag("transaction-filters-screen"),
        containerColor = MaterialTheme.colorScheme.background,
        contentWindowInsets = WindowInsets.safeDrawing,
        topBar = {
            TextButton(onClick = browser::cancelFilters, enabled = !state.busy, contentPadding = PaddingValues(0.dp),
                modifier = Modifier.statusBarsPadding().padding(horizontal = 20.dp).fillMaxWidth()
                    .cuenoraFocus().heightIn(min = 48.dp).testTag("history-filter-back")
                    .semantics { contentDescription = "Back to transactions" }) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = null, modifier = Modifier.size(20.dp))
                    Text("Transactions", style = MaterialTheme.typography.bodyMedium)
                }
            }
        }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding()
            .verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(24.dp)) {
            Text("Filter transactions", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            CuenoraDateField("From date", state.draft.from, { browser.updateDraft(state.draft.copy(from = it)) },
                Modifier.testTag("history-from"), enabled = !state.busy, optional = true)
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                CuenoraDateField("Through date", state.draft.to, { browser.updateDraft(state.draft.copy(to = it)) },
                    Modifier.testTag("history-to"), enabled = !state.busy, optional = true)
                Text("Inclusive business dates; leave blank for no date restriction.",
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            val accountLabels = state.accounts.map { row ->
                val account = row.account
                "${account.name.value} · ${account.currency.name} · ${if (account.status == AccountStatus.ARCHIVED) "Archived" else "Active"}"
            }
            val selectedAccount = when (val account = state.draft.account) {
                null -> "All accounts"
                TransactionAccountFilter.Accountless -> "No account assigned"
                is TransactionAccountFilter.Linked -> {
                    val index = state.accounts.indexOfFirst { it.account.id == account.accountId }
                    if (index >= 0) accountLabels[index] else state.accountLabels[account.accountId] ?: "Account unavailable"
                }
            }
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                HistoryFilterSelector("Account", selectedAccount, listOf("All accounts", "No account assigned") + accountLabels,
                    "history-filter-account", !state.busy, onPopupVisibilityChange) { index ->
                    browser.updateDraft(state.draft.copy(account = when (index) {
                        0 -> null
                        1 -> TransactionAccountFilter.Accountless
                        else -> TransactionAccountFilter.Linked(state.accounts[index - 2].account.id)
                    }))
                }
                Text("Includes entries with no account assigned.", style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            val accountless = state.draft.account == TransactionAccountFilter.Accountless
            OutlinedButton(onClick = { browser.updateDraft(state.draft.copy(account = TransactionAccountFilter.Accountless)) },
                enabled = !state.busy, colors = ButtonDefaults.outlinedButtonColors(
                    containerColor = if (accountless) MaterialTheme.colorScheme.surfaceVariant else Color.Transparent),
                modifier = Modifier.fillMaxWidth().cuenoraFocus().heightIn(min = 56.dp)
                    .testTag("history-filter-accountless").semantics { selected = accountless }) {
                Text("Show only unassigned entries")
            }
            if (state.accountCursor != null) TextButton(onClick = { submit { browser.loadChoices(more = true) } },
                enabled = !state.busy, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) { Text("More account choices") }
            HistoryFilterSelector("Type", state.draft.kind?.let(::transactionKind) ?: "All types",
                listOf("All types") + TransactionKind.entries.map(::transactionKind),
                "history-filter-kind", !state.busy, onPopupVisibilityChange) { index ->
                browser.updateDraft(state.draft.copy(kind = TransactionKind.entries.getOrNull(index - 1)))
            }
            HistoryFilterSelector("Currency", state.draft.currency?.name ?: "All currencies",
                listOf("All currencies") + Currency.entries.map { it.name },
                "history-filter-currency", !state.busy, onPopupVisibilityChange) { index ->
                browser.updateDraft(state.draft.copy(currency = Currency.entries.getOrNull(index - 1)))
            }
            val categoryIndex = state.categories.indexOfFirst { it.id == state.draft.category }
            val categoryLabels = state.categories.map { "${it.name} · ${transactionKind(it.kind)}" }
            HistoryFilterSelector("Category", if (state.draft.category == null) "All categories"
                else if (categoryIndex >= 0) categoryLabels[categoryIndex] else "Category unavailable",
                listOf("All categories") + categoryLabels, "history-filter-category", !state.busy, onPopupVisibilityChange) { index ->
                browser.updateDraft(state.draft.copy(category = if (index == 0) null else state.categories[index - 1].id))
            }
            if (state.choicesFailure) {
                Text("Filter choices unavailable. Existing selections are unchanged.")
                TextButton(onClick = { submit { browser.loadChoices() } }, enabled = !state.busy,
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) { Text("Retry filter choices") }
            }
            Text("Dates refer to when the activity happened, not when a record was edited.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            state.filterError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            CuenoraAction("Apply filters", { submit { browser.applyFilters() } }, Modifier.testTag("history-filter-apply"),
                unavailableReason = if (state.busy) "Loading filter choices…" else null)
            OutlinedButton(onClick = browser::resetDraft, enabled = !state.busy,
                modifier = Modifier.fillMaxWidth().cuenoraFocus().heightIn(min = 56.dp).testTag("history-filter-reset")) { Text("Reset filters") }
            OutlinedButton(onClick = browser::cancelFilters, enabled = !state.busy,
                modifier = Modifier.fillMaxWidth().cuenoraFocus().heightIn(min = 56.dp).testTag("history-filter-cancel")) { Text("Cancel") }
        }
    }
}

@Composable
private fun HistoryFilterSelector(label: String, selected: String, choices: List<String>, tag: String,
    enabled: Boolean, onPopupVisibilityChange: (Boolean) -> Unit, choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().testTag(tag), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Box {
            OutlinedButton(onClick = { expanded = true }, enabled = enabled, shape = MaterialTheme.shapes.small,
                modifier = Modifier.fillMaxWidth().cuenoraFocus().heightIn(min = 56.dp)
                    .semantics { contentDescription = "$label: $selected" }) {
                Text(selected, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                Icon(Icons.Default.ArrowDropDown, contentDescription = null)
            }
            DropdownMenu(expanded = expanded && enabled, onDismissRequest = { expanded = false }) {
                // Keep the parent modal protected until this popup's exit animation is disposed.
                DisposableEffect(Unit) {
                    onPopupVisibilityChange(true)
                    onDispose { onPopupVisibilityChange(false) }
                }
                choices.forEachIndexed { index, choice ->
                    DropdownMenuItem(text = { Text(choice) }, onClick = { expanded = false; choose(index) })
                }
            }
        }
    }
}
