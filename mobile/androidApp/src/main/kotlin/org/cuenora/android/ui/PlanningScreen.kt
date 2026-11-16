package org.cuenora.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Warning
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
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
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp)) {
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
            HomeBalances(data, accounts)
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Plan upcoming payments", style = MaterialTheme.typography.titleLarge,
                    modifier = Modifier.semantics { heading() })
                OutlinedButton(onClick = {
                    draft = state.selectedThrough?.let(::planningDate) ?: planningDate(data.through)
                    editingDate = true
                }, enabled = !state.loading, shape = MaterialTheme.shapes.small,
                    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                    modifier = Modifier.fillMaxWidth().testTag("choose-planning-date")) {
                    Text("Through ${planningDate(data.through)}")
                }
                if (!data.complete) CuenoraNotice("Planning is incomplete",
                    "Missing payment coverage is not treated as zero.", NoticeTone.Attention)
                if (data.accounts.isEmpty()) Text("Income and expenses can still be recorded without an account.")
                data.accounts.forEachIndexed { index, account -> HomeAccountPlanning(account, index) }
                Text("Recorded balance minus pending and projected payments. An estimate, not available-to-spend money.",
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (upcomingContent != null || recurrences != null || data.unassigned.any {
                    it.pendingTotal != "0.00" || it.projectedTotal != "0.00" || !it.complete
                }) {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("Upcoming payments", style = MaterialTheme.typography.titleLarge,
                        modifier = Modifier.semantics { heading() })
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
                    if (upcomingContent == null && recurrences != null) {
                        OutlinedButton(onClick = recurrences, shape = MaterialTheme.shapes.small,
                            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
                            modifier = Modifier.fillMaxWidth().testTag("review-planned-payments")) {
                            Text("Review planned payments")
                        }
                        Text("Overdue payments stay pending until you confirm an actual payment.",
                            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
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

/** BF49A owns the grouping; the current native theme owns color and typography. */
@Composable
private fun HomeBalances(data: AccountPlanning, accounts: (() -> Unit)?) {
    Column(Modifier.fillMaxWidth().testTag("planning-balance-summary"), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Recorded balances", style = MaterialTheme.typography.titleLarge,
            modifier = Modifier.semantics { heading() })
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            val availableWidth = maxWidth
            FlowRow(horizontalArrangement = Arrangement.spacedBy(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                Currency.entries.forEach { currency ->
                    val group = data.accounts.filter { it.currency == currency }
                    if (group.isNotEmpty()) {
                        Column(Modifier.widthIn(max = availableWidth).width(IntrinsicSize.Max).testTag("home-currency-${currency.name}"),
                            verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(currency.name, style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(ExactDecimal.sum(group.map { it.currentBalance }), fontFamily = UiFont,
                                fontWeight = FontWeight.Medium, fontSize = 24.sp, lineHeight = 36.sp)
                        }
                    }
                }
            }
        }
        data.accounts.filter { it.type == AccountType.SAVINGS }.groupBy { it.currency }.forEach { (currency, savings) ->
            FinancialMetric("Savings · included", currency.name, ExactDecimal.sum(savings.map { it.currentBalance }))
        }
        if (data.accounts.isEmpty()) Text("No accounts yet")
        Text("Recorded money only · no currency conversion", style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant)
        accounts?.let { OutlinedButton(onClick = it, shape = MaterialTheme.shapes.small,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline), modifier = Modifier.fillMaxWidth()) { Text("View accounts") } }
    }
}

@Composable
private fun HomeAccountPlanning(account: PlanningAccount, index: Int) {
    val shortfall = account.planningDifference?.startsWith('-') == true
    val noPending = account.complete && account.pendingTotal == "0.00" && account.projectedTotal == "0.00"
    if (noPending) {
        Column(Modifier.fillMaxWidth().testTag("home-no-pending-$index"), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            FinancialMetric(account.name, account.currency.name, account.currentBalance)
            if (account.type == AccountType.SAVINGS || account.status == AccountStatus.ARCHIVED) HomeAccountMetadata(account)
            Text("No pending payments through this date.", style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    } else {
        Surface(Modifier.fillMaxWidth(), shape = MaterialTheme.shapes.medium,
            color = if (shortfall) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.colorScheme.surface,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(account.name, style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f).semantics { heading() })
                    if (shortfall) Icon(Icons.Default.Warning, contentDescription = "Planning shortfall",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.testTag("home-shortfall-indicator-$index"))
                }
                HomeAccountMetadata(account)
                FinancialMetric("Recorded balance", account.currency.name, account.currentBalance)
                FinancialMetric("Pending payments", account.currency.name, account.pendingTotal)
                if (account.projectedTotal != "0.00") FinancialMetric("Projected payments", account.currency.name, account.projectedTotal)
                HorizontalDivider()
                val difference = account.planningDifference
                if (account.complete && difference != null) {
                    CuenoraAmount(if (shortfall) "Estimated shortfall" else "Estimated remaining", account.currency.name, difference)
                }
            }
        }
    }
}

@Composable
private fun HomeAccountMetadata(account: PlanningAccount) {
    Text("${if (account.type == AccountType.SAVINGS) "Savings" else "Normal"} · ${if (account.status == AccountStatus.ARCHIVED) "Archived account" else "Active account"} · ${account.currency.name}",
        style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

private fun planningDate(date: BusinessDate) = "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"

private fun parsePlanningDate(value: String): BusinessDate {
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(value))
    return BusinessDate(value.take(4).toInt(), value.substring(5, 7).toInt(), value.takeLast(2).toInt())
}
