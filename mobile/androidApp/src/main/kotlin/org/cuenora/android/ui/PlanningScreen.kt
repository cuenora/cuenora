package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.cuenora.shared.domain.*
import org.cuenora.shared.planning.*

@Composable
fun PlanningScreen(
    session: PlanningSession,
    submit: (suspend () -> Unit) -> Unit,
    accounts: (() -> Unit)? = null,
    recurrences: (() -> Unit)? = null,
    upcomingContent: (@Composable () -> Unit)? = null,
    profileTimeZone: String? = null,
) {
    val state by session.state.collectAsState()
    val locale = LocalLocale.current.platformLocale
    var editingDate by remember(session) { mutableStateOf(false) }
    var draft by remember(session) { mutableStateOf("") }
    LaunchedEffect(session) { submit { session.loadIfNeeded() } }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp)) {
        if (state.loading) CuenoraLoading(if (state.loaded) "Refreshing balances…" else "Loading balances…")
        state.failure?.let { failure ->
            CuenoraNotice("Account planning is unavailable", when (failure) {
                PlanningLoadFailure.AccessDenied -> "Sign in again or check email verification."
                PlanningLoadFailure.InvalidResponse -> "The response could not be read safely. Try refreshing."
                PlanningLoadFailure.Unavailable -> "Check your connection and try again. No records were changed."
            }, NoticeTone.Error)
            TextButton(onClick = { submit { session.refresh() } }, enabled = !state.loading) { Text("Try again") }
        }
        val data = state.planning
        if (data != null) {
            FinancialSection("Recorded balances", Modifier.testTag("planning-balance-summary"), emphasis = true) {
                Currency.entries.forEach { currency ->
                    val group = data.accounts.filter { it.currency == currency }
                    if (group.isNotEmpty()) {
                        CuenoraAmount(currency.name, currency.name, ExactDecimal.sum(group.map { it.currentBalance }))
                        val savings = group.filter { it.type == AccountType.SAVINGS }
                        if (savings.isNotEmpty()) FinancialMetric("Savings · included", currency.name,
                            ExactDecimal.sum(savings.map { it.currentBalance }))
                    }
                }
                if (data.accounts.isEmpty()) Text("No accounts yet")
                Text("Recorded money only · no currency conversion", style = MaterialTheme.typography.bodySmall)
                accounts?.let { TextButton(onClick = it) { Text("View accounts") } }
            }
            Text("Plan upcoming payments", style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.semantics { heading() })
            OutlinedButton(onClick = {
                draft = state.selectedThrough?.let(::planningDate) ?: planningDate(data.through)
                editingDate = true
            }, enabled = !state.loading, modifier = Modifier.fillMaxWidth().testTag("choose-planning-date")) {
                Text("Through ${planningDate(data.through)}")
            }
            if (upcomingContent == null && recurrences != null) {
                OutlinedButton(onClick = recurrences, modifier = Modifier.fillMaxWidth().testTag("review-planned-payments")) {
                    Text("Review planned payments")
                }
                Text("Overdue payments stay pending until you confirm an actual payment.", style = MaterialTheme.typography.bodySmall)
            }
            if (!data.complete) CuenoraNotice("Planning is incomplete",
                "Missing payment coverage is not treated as zero.", NoticeTone.Attention)
            if (data.accounts.isEmpty()) Text("Income and expenses can still be recorded without an account.")
            data.accounts.forEach { account ->
                FinancialSection(account.name, emphasis = account.planningDifference?.startsWith('-') == true) {
                    Text("${if (account.type == AccountType.SAVINGS) "Savings" else "Normal"} · ${if (account.status == AccountStatus.ARCHIVED) "Archived account" else "Active account"} · ${account.currency.name}",
                        style = MaterialTheme.typography.bodySmall)
                    FinancialMetric("Recorded balance", account.currency.name, account.currentBalance)
                    FinancialMetric("Pending payments", account.currency.name, account.pendingTotal)
                    if (account.projectedTotal != "0.00") FinancialMetric("Projected payments", account.currency.name, account.projectedTotal)
                    HorizontalDivider()
                    val difference = account.planningDifference
                    if (account.complete && difference != null) {
                        CuenoraAmount(if (difference.startsWith('-')) "Estimated shortfall" else "Estimated remaining",
                            account.currency.name, difference)
                    }
                }
            }
            Text("Recorded balance minus pending and projected payments. An estimate, not available-to-spend money.",
                style = MaterialTheme.typography.bodySmall)
            if (upcomingContent != null) upcomingContent()
            if (data.unassigned.any { it.pendingTotal != "0.00" || it.projectedTotal != "0.00" || !it.complete }) {
                FinancialSection("Unassigned payment commitments") {
                    Text("Not deducted from any account.", style = MaterialTheme.typography.bodySmall)
                    data.unassigned.filter { it.pendingTotal != "0.00" || it.projectedTotal != "0.00" || !it.complete }.forEach { row ->
                        FinancialMetric("${row.currency.name} pending", row.currency.name, row.pendingTotal)
                        if (row.projectedTotal != "0.00") FinancialMetric("${row.currency.name} projected", row.currency.name, row.projectedTotal)
                        if (!row.complete) Text("${row.currency.name} coverage is incomplete.")
                    }
                }
            }
            val snapshotTime = formatSnapshotTimestamp(data.asOf, profileTimeZone, locale)
            Text(snapshotTime?.let { "Updated: $it" } ?: "Update time unavailable", style = MaterialTheme.typography.bodySmall)
            TextButton(onClick = { submit { session.refresh() } }, enabled = !state.loading) { Text("Refresh planning") }
        } else if (state.loaded && state.failure == null) Text("No planning data is available.")
    }
    if (editingDate) {
        val parsed = runCatching { if (draft.isBlank()) null else parsePlanningDate(draft) }.getOrNull()
        AlertDialog(onDismissRequest = { if (!state.loading) editingDate = false }, title = { Text("Plan through a date") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    CuenoraDateField("Through", draft, { draft = it },
                        modifier = Modifier.fillMaxWidth().testTag("planning-date"))
                    TextButton(onClick = { editingDate = false; submit { session.applyThrough(null) } }, enabled = !state.loading) {
                        Text("Use server default")
                    }
                }
            }, confirmButton = {
                TextButton(onClick = { editingDate = false; submit { session.applyThrough(parsed) } },
                    enabled = !state.loading && (draft.isBlank() || parsed != null), modifier = Modifier.testTag("apply-planning-date")) { Text("Apply date") }
            }, dismissButton = { TextButton(onClick = { editingDate = false }, enabled = !state.loading) { Text("Cancel") } })
    }
}

private fun planningDate(date: BusinessDate) = "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"

private fun parsePlanningDate(value: String): BusinessDate {
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(value))
    return BusinessDate(value.take(4).toInt(), value.substring(5, 7).toInt(), value.takeLast(2).toInt())
}
