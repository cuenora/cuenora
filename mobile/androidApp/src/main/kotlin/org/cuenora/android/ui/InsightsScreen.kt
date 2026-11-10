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
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.insights.*

@Composable
fun InsightsScreen(
    session: InsightsSession,
    submit: (suspend () -> Unit) -> Unit,
    onTransactions: (dateFrom: String, dateTo: String, categoryId: CategoryId?) -> Unit,
) {
    val state by session.state.collectAsState()
    LaunchedEffect(session) { submit { session.loadIfNeeded() } }
    Column(Modifier.fillMaxSize().testTag("insights-screen").verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Insights", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
        Text("Realized income and expenses use business dates, include accountless activity, and exclude transfers, openings, voided events, planned payments and forecasts.")
        state.range?.let { range -> Text("Reporting period: ${range.dateFrom} through ${range.dateTo} · inclusive") }
        if (state.editingDates) {
            OutlinedTextField(state.draftFrom, { session.changeDateDraft(dateFrom = it) }, label = { Text("From (YYYY-MM-DD)") }, modifier = Modifier.testTag("insights-date-from"))
            OutlinedTextField(state.draftTo, { session.changeDateDraft(dateTo = it) }, label = { Text("Through (YYYY-MM-DD)") }, modifier = Modifier.testTag("insights-date-to"))
            state.dateError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { submit { session.applyDateDraft() } }, enabled = !state.busy) { Text("Apply dates") }
                TextButton(onClick = session::cancelDateDraft, enabled = !state.busy) { Text("Cancel") }
            }
        } else {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = session::editDates, enabled = !state.busy) { Text("Change dates") }
                TextButton(onClick = { submit { session.resetDateRange() } }, enabled = !state.busy) { Text("Current month") }
                TextButton(onClick = { submit { session.refresh() } }, enabled = !state.busy) { Text("Refresh insights") }
            }
        }
        if (state.busy) CuenoraLoading("Reading three separate reports…")
        state.error?.let { failure ->
            CuenoraNotice("Insights unavailable", when ((failure as? org.cuenora.shared.network.ApiFailure.Http)?.status) {
                401 -> "Sign in again to read financial reports."
                403 -> "Report access is unavailable. Confirm your profile and retry."
                else -> "Check your connection and try again. No financial records were changed."
            }, NoticeTone.Error)
            TextButton(onClick = { submit { session.refresh() } }, enabled = !state.busy) { Text("Try again") }
        }
        state.report?.let { report ->
            report.currencies.forEach { row ->
                FilterChip(selected = state.selectedCurrency == row.currency, onClick = { session.selectCurrency(row.currency) },
                    label = { Text(row.currency.name) })
            }
            report.currencies.firstOrNull { it.currency == state.selectedCurrency }?.let { row ->
                Text("${row.currency.name} summary", style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                Text("Income · ${row.currency.name} ${row.income}")
                Text("Expenses · ${row.currency.name} ${row.expenses}")
                Text("Net flow · ${row.currency.name} ${row.netFlow}")
                Text("Summary read separately from monthly trends and category distribution.")
                Text("Totals are separate by currency. Savings balances are not added to realized flow.")
                TextButton(onClick = { onTransactions(report.range.dateFrom, report.range.dateTo, null) }) {
                    Text("Open broader transaction history for these dates")
                }
                Text("Monthly trends", style = MaterialTheme.typography.titleLarge)
                Text("Monthly trends are a separate report read.")
                row.months.forEach { month ->
                    TextButton(onClick = { onTransactions(month.dateFrom, month.dateTo, null) }, modifier = Modifier.fillMaxWidth().testTag("insights-month-${month.month}")) {
                        Text("${month.month} · ${row.currency.name} ${month.income} income · ${row.currency.name} ${month.expenses} expenses · ${row.currency.name} ${month.netFlow} net")
                    }
                }
                Text("Expense categories", style = MaterialTheme.typography.titleLarge)
                Text("Category distribution is a separate report read · total ${row.currency.name} ${row.distributionExpenses}.")
                if (row.distributionExpenses == "0.00") Text("No percentage distribution for zero category spending.")
                row.categories.forEach { category ->
                    val title = category.name ?: "Uncategorized"
                    TextButton(onClick = {
                        // History cannot filter currency, transaction kind, or the uncategorized bucket.
                        onTransactions(report.range.dateFrom, report.range.dateTo, category.id)
                    }, modifier = Modifier.fillMaxWidth().testTag("insights-category-${category.code ?: "uncategorized"}")) {
                        Text("$title · ${row.currency.name} ${category.expenses} · ${category.percentage?.let { "$it%" } ?: "no percentage"} · opens broader history")
                    }
                }
                Text("Category links open date-filtered transaction history; the history view may include other currencies and transaction kinds. Uncategorized opens all categories.")
            }
        }
    }
}
