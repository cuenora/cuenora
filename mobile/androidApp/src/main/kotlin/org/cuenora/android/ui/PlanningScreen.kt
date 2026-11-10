package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
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
) {
    val state by session.state.collectAsState()
    var draft by remember(session, state.selectedThrough) { mutableStateOf(state.selectedThrough?.let(::planningDate) ?: "") }
    LaunchedEffect(session) { submit { session.loadIfNeeded() } }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Home", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
        Text("Recorded balances and known payment commitments, kept separate.")
        OutlinedTextField(draft, { draft = it }, label = { Text("Planning cutoff (YYYY-MM-DD)") }, singleLine = true,
            modifier = Modifier.fillMaxWidth().testTag("planning-date"))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            val parsed = runCatching { if (draft.isBlank()) null else parsePlanningDate(draft) }.getOrNull()
            Button(onClick = { submit { session.applyThrough(parsed) } }, enabled = !state.loading && (draft.isBlank() || parsed != null), modifier = Modifier.testTag("apply-planning-date")) { Text("Apply date") }
            TextButton(onClick = { draft = state.selectedThrough?.let(::planningDate) ?: "" }, enabled = !state.loading) { Text("Cancel") }
            TextButton(onClick = { draft = ""; submit { session.applyThrough(null) } }, enabled = !state.loading) { Text("Use server default") }
        }
        TextButton(onClick = { submit { session.refresh() } }, enabled = !state.loading) { Text("Refresh planning") }
        if (state.loading) CuenoraLoading(if (state.loaded) "Refreshing account planning…" else "Loading account planning…")
        state.failure?.let { failure ->
            CuenoraNotice("Account planning is unavailable", when (failure) {
                PlanningLoadFailure.AccessDenied -> "Your session or account access is unavailable. Sign in again or check email verification."
                PlanningLoadFailure.InvalidResponse -> "The planning response could not be read safely. Try refreshing."
                PlanningLoadFailure.Unavailable -> "Current planning data could not be loaded. No financial records were changed."
            }, NoticeTone.Error)
            TextButton(onClick = { submit { session.refresh() } }, enabled = !state.loading) { Text("Try again") }
        }
        val data = state.planning
        if (data != null) {
            Text("Snapshot as of ${data.asOf} · cutoff ${planningDate(data.through)}")
            if (!data.complete) CuenoraNotice("Planning is incomplete", "Some payment coverage is unavailable. Missing amounts and differences are not treated as zero.", NoticeTone.Attention)
            if (data.accounts.isEmpty()) {
                Text("No accounts yet", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                Text("There are no account balances to plan against. Income and expenses can still be recorded without an account.")
            } else {
                Currency.values().forEach { currency ->
                    val inCurrency = data.accounts.filter { it.currency == currency }
                    if (inCurrency.isNotEmpty()) {
                        Text("${currency.name} accounts", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                        CuenoraAmount("Total recorded balance", currency.name, ExactDecimal.sum(inCurrency.map { it.currentBalance }))
                        val savings = inCurrency.filter { it.type == AccountType.SAVINGS }
                        if (savings.isNotEmpty()) CuenoraAmount("Savings (included in total)", currency.name, ExactDecimal.sum(savings.map { it.currentBalance }))
                    }
                }
                data.accounts.forEach { account ->
                    HorizontalDivider()
                    Text(account.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
                    Text("${if (account.type == AccountType.SAVINGS) "Savings" else "Normal"} · ${if (account.status == AccountStatus.ARCHIVED) "Archived account" else "Active account"} · ${account.currency.name}")
                    CuenoraAmount("Recorded balance", account.currency.name, account.currentBalance)
                    CuenoraAmount("Pending payments through cutoff", account.currency.name, account.pendingTotal)
                    CuenoraAmount("Projected payments through cutoff", account.currency.name, account.projectedTotal)
                    Text("Planning difference = recorded balance − pending payments − projected payments")
                    val difference = account.planningDifference
                    if (account.complete && difference != null) {
                        CuenoraAmount(if (difference.startsWith('-')) "Estimated shortfall" else "Estimated remaining after known commitments", account.currency.name, difference)
                        Text("This is an estimate of known commitments, not a guarantee that funds are available.")
                    } else Text("Planning difference unavailable because this account's payment coverage is incomplete.")
                }
            }
            Text("Unassigned payment commitments", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
            Text("These amounts are not assigned to an account and do not reduce account differences.")
            data.unassigned.forEach { row ->
                CuenoraAmount("${row.currency.name} pending", row.currency.name, row.pendingTotal)
                CuenoraAmount("${row.currency.name} projected", row.currency.name, row.projectedTotal)
                if (!row.complete) Text("${row.currency.name} unassigned coverage is incomplete.")
            }
            if (accounts != null) TextButton(onClick = accounts) { Text("View accounts") }
            if (recurrences != null) TextButton(onClick = recurrences) { Text("View recurring payments") }
        } else if (state.loaded && state.failure == null) {
            Text("No planning data is available.")
        }
    }
}

private fun planningDate(date: BusinessDate) = "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"

private fun parsePlanningDate(value: String): BusinessDate {
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(value))
    return BusinessDate(value.take(4).toInt(), value.substring(5, 7).toInt(), value.takeLast(2).toInt())
}
