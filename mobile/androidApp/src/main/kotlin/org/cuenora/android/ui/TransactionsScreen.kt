package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import org.cuenora.shared.repository.*

@Composable
fun TransactionsScreen(browser: TransactionBrowser, submit: (suspend () -> Unit) -> Unit, record: () -> Unit) {
    val state by browser.state.collectAsState()
    LaunchedEffect(browser) { browser.loadIfNeeded() }
    LazyColumn(Modifier.fillMaxSize().testTag("transaction-list"), contentPadding = PaddingValues(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)) {
        item {
            Text("Transactions", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            Text("Recorded activity, newest business date first. No planned payments or combined-currency totals.")
            CuenoraAction("Record income or expense", record, Modifier.testTag("record-from-transactions"))
            TextButton(onClick = { browser.openFilters(); submit { browser.loadChoices() } }, enabled = !state.busy) { Text("Filters") }
            Text(filterSummary(state))
            TextButton(onClick = { submit { browser.refresh() } }, enabled = !state.busy) { Text("Refresh transactions") }
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
        items(state.items, key = { it.transaction.id.toString() }) { record ->
            val event = record.transaction
            Column(Modifier.fillMaxWidth().testTag("transaction-${event.id}"), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(transactionKind(event.kind), style = MaterialTheme.typography.titleLarge)
                Text("${historyDate(event.occurredOn)} · ${if (event.state == TransactionState.VOIDED) "Voided" else "Posted"}")
                CuenoraAmount(if (event.kind == TransactionKind.TRANSFER) "Sent" else "Recorded amount", event.amount.currency.name, accountAmount(event.amount))
                event.receivedAmount?.let { CuenoraAmount("Received", it.currency.name, accountAmount(it)) }
                Text(event.accountId?.let { "Account: ${accountLabel(it, state)}" } ?: "No account assigned")
                event.destinationAccountId?.let { Text("Destination: ${accountLabel(it, state)}") }
                event.categoryId?.let { id -> Text("Category: " + (state.categories.firstOrNull { it.id == id }?.name ?: "Category unavailable")) }
                event.description?.let { Text(it) }
                if (event.kind == TransactionKind.TRANSFER || event.kind == TransactionKind.OPENING_BALANCE)
                    Text("Not income or spending.")
                HorizontalDivider()
            }
        }
        if (state.loaded && state.nextCursor != null && state.failure == null) item {
            TextButton(onClick = { submit { browser.loadMore() } }, enabled = !state.busy) { Text("Load more transactions") }
        }
    }
    if (state.filtersOpen) {
        BackHandler { browser.cancelFilters() }
        AlertDialog(onDismissRequest = browser::cancelFilters, title = { Text("Transaction filters") },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Inclusive business dates; leave blank for no date restriction.")
                    OutlinedTextField(state.draft.from, { browser.updateDraft(state.draft.copy(from = it)) },
                        label = { Text("From (YYYY-MM-DD)") }, enabled = !state.busy, modifier = Modifier.testTag("history-from"))
                    OutlinedTextField(state.draft.to, { browser.updateDraft(state.draft.copy(to = it)) },
                        label = { Text("To (YYYY-MM-DD)") }, enabled = !state.busy, modifier = Modifier.testTag("history-to"))
                    Text("Account")
                    FilterChoice("All accounts", state.draft.account == null, !state.busy) { browser.updateDraft(state.draft.copy(account = null)) }
                    FilterChoice("No account assigned", state.draft.account == TransactionAccountFilter.Accountless, !state.busy) {
                        browser.updateDraft(state.draft.copy(account = TransactionAccountFilter.Accountless))
                    }
                    state.accounts.forEach { row ->
                        val account = row.account
                        FilterChoice("${account.name.value} · ${account.currency.name} · ${if (account.status == AccountStatus.ARCHIVED) "Archived" else "Active"}",
                            state.draft.account == TransactionAccountFilter.Linked(account.id), !state.busy) {
                            browser.updateDraft(state.draft.copy(account = TransactionAccountFilter.Linked(account.id)))
                        }
                    }
                    if (state.accountCursor != null) TextButton(onClick = { submit { browser.loadChoices(more = true) } }, enabled = !state.busy) { Text("More account choices") }
                    Text("Category")
                    FilterChoice("All categories", state.draft.category == null, !state.busy) { browser.updateDraft(state.draft.copy(category = null)) }
                    state.categories.forEach { category ->
                        FilterChoice("${category.name} · ${transactionKind(category.kind)}", state.draft.category == category.id, !state.busy) {
                            browser.updateDraft(state.draft.copy(category = category.id))
                        }
                    }
                    if (state.choicesFailure) {
                        Text("Filter choices unavailable. Existing selections are unchanged.")
                        TextButton(onClick = { submit { browser.loadChoices() } }, enabled = !state.busy) { Text("Retry filter choices") }
                    }
                    if (state.busy) Text("Loading filter choices…")
                    state.filterError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                    TextButton(onClick = browser::resetDraft, enabled = !state.busy) { Text("Reset filters") }
                }
            }, confirmButton = { TextButton(onClick = { submit { browser.applyFilters() } }, enabled = !state.busy) { Text("Apply filters") } },
            dismissButton = { TextButton(onClick = browser::cancelFilters, enabled = !state.busy) { Text("Cancel") } })
    }
}

@Composable
private fun FilterChoice(label: String, selected: Boolean, enabled: Boolean, choose: () -> Unit) {
    TextButton(onClick = choose, enabled = enabled, modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
        Text((if (selected) "Selected: " else "") + label)
    }
}
private fun accountLabel(id: AccountId, state: TransactionBrowserState) = state.accountLabels[id] ?: "Account unavailable"
private fun filterSummary(state: TransactionBrowserState): String {
    val draft = state.applied
    return "Dates: ${draft.from.ifEmpty { "any" }} to ${draft.to.ifEmpty { "any" }} · " +
    when (val account = draft.account) {
        null -> "All accounts"
        TransactionAccountFilter.Accountless -> "No account assigned"
        is TransactionAccountFilter.Linked -> "Account ${accountLabel(account.accountId, state)}"
    } + if (draft.category == null) " · All categories" else " · Category ${state.categories.firstOrNull { it.id == draft.category }?.name ?: "selected"}"
}
private fun historyDate(date: BusinessDate) = "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"
private fun transactionKind(kind: TransactionKind) = when (kind) {
    TransactionKind.INCOME -> "Income"
    TransactionKind.EXPENSE -> "Expense"
    TransactionKind.TRANSFER -> "Transfer"
    TransactionKind.OPENING_BALANCE -> "Opening balance"
    TransactionKind.INTEREST_INCOME -> "Credited interest"
}
