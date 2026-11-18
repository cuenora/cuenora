package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import java.text.SimpleDateFormat
import java.util.Date
import java.util.GregorianCalendar
import java.util.TimeZone
import java.util.Locale
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*

@Composable
fun TransactionsScreen(browser: TransactionBrowser, submit: (suspend () -> Unit) -> Unit, record: () -> Unit) =
    TransactionHistoryScreen(browser, submit, null, null, record)

@Composable
fun TransactionsScreen(
    browser: TransactionBrowser,
    submit: (suspend () -> Unit) -> Unit,
    voidSession: org.cuenora.shared.history.TransactionVoidSession,
    onVoided: (() -> Unit)? = null,
    record: () -> Unit,
) = TransactionHistoryScreen(browser, submit, voidSession, onVoided, record)

@Composable
private fun TransactionHistoryScreen(
    browser: TransactionBrowser,
    submit: (suspend () -> Unit) -> Unit,
    voidSession: org.cuenora.shared.history.TransactionVoidSession?,
    onVoided: (() -> Unit)?,
    record: () -> Unit,
) {
    val state by browser.state.collectAsState()
    LaunchedEffect(browser) { browser.loadIfNeeded() }
    LazyColumn(Modifier.fillMaxSize().testTag("transaction-list"), contentPadding = PaddingValues(horizontal = 20.dp, vertical = 24.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp)) {
        item {
            Text("Transactions", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
        }
        item {
            CuenoraAction("Record activity", record, Modifier.testTag("record-from-transactions"))
        }
        item {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(
                    onClick = { browser.openFilters(); submit { browser.loadChoices() } },
                    enabled = !state.busy,
                    modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp).testTag("transactions-filters"),
                ) { Text("Filters") }
                Text(filterSummary(state), style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (state.busy) item { CuenoraLoading("Loading transactions…") }
        state.failure?.let { failure -> item {
            CuenoraNotice("Transactions unavailable", when ((failure as? org.cuenora.shared.network.ApiFailure.Http)?.status) {
                401 -> "Sign in again to read your transactions."
                403 -> "Access unavailable. Check email verification from Home and retry."
                else -> "Check your connection and try again. No financial records were changed."
            }, NoticeTone.Error)
            TextButton(onClick = { submit { if (state.loaded) browser.loadMore() else browser.refresh() } }, enabled = !state.busy) {
                Text(if (state.loaded) "Retry more transactions" else "Try again")
            }
        } }
        if (state.loaded && state.items.isEmpty()) item {
            Text(if (state.applied == HistoryFilterDraft()) "No recorded transactions yet" else "No matching transactions",
                style = MaterialTheme.typography.titleLarge)
            Text("Try different filters or record income or an expense.")
        }
        itemsIndexed(state.items, key = { _, row -> row.transaction.id.toString() }) { index, row ->
            val event = row.transaction
            Column(Modifier.fillMaxWidth().testTag("transaction-${event.id}"),
                verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (index == 0 || state.items[index - 1].transaction.occurredOn != event.occurredOn) {
                    Text(overviewDate(event.occurredOn), style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(bottom = 10.dp).semantics { heading() })
                }
                Text(event.description?.takeIf { it.isNotBlank() } ?: when (event.kind) {
                    TransactionKind.TRANSFER -> "${event.accountId?.let { accountLabel(it, state) }} → ${event.destinationAccountId?.let { accountLabel(it, state) }}"
                    else -> event.categoryId?.let { id -> state.categories.firstOrNull { it.id == id }?.name }
                        ?: transactionKind(event.kind)
                }, style = MaterialTheme.typography.titleMedium)
                if (event.kind == TransactionKind.TRANSFER) {
                    Text("Sent", style = MaterialTheme.typography.labelMedium)
                    Text("${event.amount.currency.name} ${accountAmount(event.amount)}",
                        style = MaterialTheme.typography.titleLarge, fontFamily = UiFont)
                    event.receivedAmount?.let {
                        Text("Received", style = MaterialTheme.typography.labelMedium)
                        Text("${it.currency.name} ${accountAmount(it)}",
                            style = MaterialTheme.typography.titleLarge, fontFamily = UiFont)
                    }
                } else {
                    val sign = when {
                        event.amount.minorUnits < 0 -> "−"
                        event.kind == TransactionKind.EXPENSE -> "−"
                        event.amount.minorUnits > 0 -> "+"
                        else -> ""
                    }
                    Text("$sign${event.amount.currency.name} ${accountAmount(event.amount).removePrefix("-")}",
                        style = MaterialTheme.typography.titleLarge, fontFamily = UiFont)
                }
                Text(transactionKind(event.kind), style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                if (event.state == TransactionState.VOIDED) Text("Voided", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(event.accountId?.let { "Account: ${accountLabel(it, state)}" } ?: "No account assigned",
                    style = MaterialTheme.typography.bodyMedium)
                event.destinationAccountId?.let { Text("Destination: ${accountLabel(it, state)}", style = MaterialTheme.typography.bodyMedium) }
                event.categoryId?.let { id -> Text("Category: " + (state.categories.firstOrNull { it.id == id }?.name ?: "Category unavailable"),
                    style = MaterialTheme.typography.bodyMedium) }
                if (event.state == TransactionState.POSTED && voidSession != null) {
                    TextButton(onClick = { voidSession.request(event) }, enabled = !state.busy,
                        modifier = Modifier.testTag("void-transaction-${event.id}")) { Text("Void transaction") }
                }
                HorizontalDivider(Modifier.padding(top = 10.dp).testTag("transaction-divider-${event.id}"))
            }
        }
        if (state.loaded && state.nextCursor != null && state.failure == null) item {
            TextButton(onClick = { submit { browser.loadMore() } }, enabled = !state.busy) { Text("Load more transactions") }
        }
        item {
            Text("Only recorded operations appear here. Upcoming payments remain in Recurrence.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            TextButton(onClick = { submit { browser.refresh() } }, enabled = !state.busy) { Text("Refresh transactions") }
            if (voidSession != null) TextButton(onClick = { submit { voidSession.restorePending() } }) { Text("Check pending void") }
        }
    }
    if (state.filtersOpen) TransactionFiltersDialog(browser, submit)
    voidSession?.let { TransactionVoidHost(it, submit, onVoided ?: {}) }
}

private fun accountLabel(id: AccountId, state: TransactionBrowserState) = state.accountLabels[id] ?: "Account unavailable"
private fun filterSummary(state: TransactionBrowserState): String {
    val draft = state.applied
    return "Dates: ${draft.from.ifEmpty { "any" }} to ${draft.to.ifEmpty { "any" }} · " +
    when (val account = draft.account) {
        null -> "All accounts"
        TransactionAccountFilter.Accountless -> "No account assigned"
        is TransactionAccountFilter.Linked -> "Account ${accountLabel(account.accountId, state)}"
    } + (if (draft.category == null) " · All categories" else
        " · Category ${state.categories.firstOrNull { it.id == draft.category }?.name ?: "selected"}") +
        (draft.kind?.let { " · ${transactionKind(it)}" } ?: "") +
        (draft.currency?.let { " · ${it.name}" } ?: "")
}
private fun overviewDate(date: BusinessDate): String {
    val locale = Locale.getDefault()
    // UTC is only a formatting anchor: a business date is not a device-zone instant.
    val calendar = GregorianCalendar(TimeZone.getTimeZone("UTC"), locale).apply {
        gregorianChange = Date(Long.MIN_VALUE)
        clear()
        set(date.year, date.month - 1, date.day)
    }
    val pattern = android.text.format.DateFormat.getBestDateTimePattern(locale, "yMMMMd")
    return SimpleDateFormat(pattern, locale).apply { timeZone = calendar.timeZone }.format(calendar.time)
}

internal fun transactionKind(kind: TransactionKind) = when (kind) {
    TransactionKind.INCOME -> "Income"
    TransactionKind.EXPENSE -> "Expense"
    TransactionKind.TRANSFER -> "Transfer"
    TransactionKind.OPENING_BALANCE -> "Opening balance"
    TransactionKind.INTEREST_INCOME -> "Credited interest"
}
