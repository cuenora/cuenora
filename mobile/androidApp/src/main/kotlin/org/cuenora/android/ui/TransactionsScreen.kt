package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
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
import org.cuenora.shared.history.ExpenseEditResult
import org.cuenora.shared.history.ExpenseEditSession
import org.cuenora.shared.repository.*

@Composable
fun TransactionsScreen(browser: TransactionBrowser, submit: (suspend () -> Unit) -> Unit, record: () -> Unit) =
    TransactionHistoryScreen(browser, submit, null, null, null, null, record)

@Composable
fun TransactionsScreen(
    browser: TransactionBrowser,
    submit: (suspend () -> Unit) -> Unit,
    voidSession: org.cuenora.shared.history.TransactionVoidSession,
    onVoided: (() -> Unit)? = null,
    expenseEdit: ExpenseEditSession? = null,
    onEdited: (() -> Unit)? = null,
    record: () -> Unit,
) = TransactionHistoryScreen(browser, submit, voidSession, onVoided, expenseEdit, onEdited, record)

@Composable
private fun TransactionHistoryScreen(
    browser: TransactionBrowser,
    submit: (suspend () -> Unit) -> Unit,
    voidSession: org.cuenora.shared.history.TransactionVoidSession?,
    onVoided: (() -> Unit)?,
    expenseEdit: ExpenseEditSession?,
    onEdited: (() -> Unit)?,
    record: () -> Unit,
) {
    val state by browser.state.collectAsState()
    LaunchedEffect(browser) { browser.loadIfNeeded() }
    val listState = rememberLazyListState()
    var selectedId by remember(browser) { mutableStateOf<TransactionId?>(null) }
    val selected = state.items.firstOrNull { it.transaction.id == selectedId }?.transaction
    var editing by remember(browser) { mutableStateOf(false) }
    var editBusy by remember(expenseEdit) { mutableStateOf(false) }
    var editPending by remember(expenseEdit) { mutableStateOf(false) }
    var editStatus by remember(expenseEdit) { mutableStateOf<String?>(null) }
    LaunchedEffect(expenseEdit) { editPending = expenseEdit?.hasPending() == true }

    fun submitEdit(action: suspend () -> ExpenseEditResult) {
        if (editBusy) return
        editBusy = true
        submit {
            try {
                when (action()) {
                    ExpenseEditResult.Saved -> {
                        editPending = false
                        editStatus = "Expense updated."
                        editing = false
                        selectedId = null
                        onEdited?.invoke()
                    }
                    ExpenseEditResult.Pending -> {
                        editPending = true
                        editStatus = "The edit is not confirmed. Retry the original edit before starting another."
                        editing = false
                    }
                    ExpenseEditResult.Rejected -> {
                        editPending = false
                        editStatus = "The server rejected this edit. Review the details before trying again."
                        onEdited?.invoke()
                    }
                    ExpenseEditResult.Unavailable -> editStatus = "Unable to verify or save the edit. Check your connection and sign-in."
                }
            } finally { editBusy = false }
        }
    }

    BackHandler(enabled = selected != null) {
        if (!editBusy) {
            if (editing) editing = false else selectedId = null
        }
    }
    if (editing && selected != null && expenseEdit != null) {
        key(selected.id) {
            ExpenseEditForm(selected, state.categories.filter { it.kind == TransactionKind.EXPENSE },
                state.busy, state.choicesFailure, editBusy, editStatus,
                onLoadCategories = { submit { browser.loadChoices() } },
                back = { editing = false },
                save = { patch -> submitEdit { expenseEdit.save(selected.id, patch) } })
        }
    } else if (selected != null) {
        Column(Modifier.fillMaxSize().testTag("transaction-detail").verticalScroll(rememberScrollState())
            .padding(20.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            TextButton(onClick = { selectedId = null }) { Text("← Transactions") }
            Text(transactionTitle(selected, state), style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() })
            Text("Recorded ${transactionKind(selected.kind).lowercase()}")
            Text(transactionAmounts(selected), style = MaterialTheme.typography.titleLarge, fontFamily = UiFont)
            Text("Business date")
            Text(overviewDate(selected.occurredOn))
            TransactionMetadata(selected, state)
            editStatus?.let { Text(it) }
            if (expenseEdit != null && editPending) {
                TextButton(onClick = { submitEdit { expenseEdit.retry() } }, enabled = !editBusy) {
                    Text("Retry original edit")
                }
            }
            if (expenseEdit != null && !editPending && selected.kind == TransactionKind.EXPENSE &&
                selected.state == TransactionState.POSTED) {
                TextButton(onClick = { editStatus = null; editing = true; submit { browser.loadChoices() } },
                    enabled = !state.busy && !editBusy, modifier = Modifier.testTag("edit-transaction")) {
                    Text("Edit record")
                }
            }
            if (selected.state == TransactionState.POSTED && voidSession != null) {
                TextButton(onClick = { voidSession.request(selected) }, enabled = !state.busy,
                    modifier = Modifier.testTag("void-transaction-${selected.id}")) { Text("Void transaction") }
            }
        }
    } else LazyColumn(Modifier.fillMaxSize().testTag("transaction-list"), state = listState, contentPadding = PaddingValues(horizontal = 20.dp, vertical = 24.dp),
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
            Column {
                if (index == 0 || state.items[index - 1].transaction.occurredOn != event.occurredOn) {
                    Text(overviewDate(event.occurredOn), style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(bottom = 10.dp).semantics { heading() })
                }
                Column(Modifier.fillMaxWidth().testTag("transaction-${event.id}")
                    .clickable(onClickLabel = "View transaction details") { selectedId = event.id }
                    .padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(transactionTitle(event, state), style = MaterialTheme.typography.titleMedium)
                    Text(transactionAmounts(event), style = MaterialTheme.typography.titleMedium, fontFamily = UiFont)
                    Text(if (event.kind == TransactionKind.TRANSFER) "Transfer · One recorded operation"
                        else transactionKind(event.kind), style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                    TransactionMetadata(event, state)
                }
                HorizontalDivider(Modifier.testTag("transaction-divider-${event.id}"))
            }
        }
        if (state.loaded && state.nextCursor != null && state.failure == null) item {
            TextButton(onClick = { submit { browser.loadMore() } }, enabled = !state.busy) { Text("Load more transactions") }
        }
        item {
            editStatus?.let { Text(it) }
            if (expenseEdit != null && editPending) {
                TextButton(onClick = { submitEdit { expenseEdit.retry() } }, enabled = !editBusy) {
                    Text("Retry original edit")
                }
            }
            Text("Only recorded operations appear here. Upcoming payments remain in Recurrence.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            TextButton(onClick = { submit { browser.refresh() } }, enabled = !state.busy) { Text("Refresh transactions") }
            if (voidSession != null) TextButton(onClick = { submit { voidSession.restorePending() } }) { Text("Check pending void") }
        }
    }
    if (state.filtersOpen) TransactionFiltersDialog(browser, submit)
    voidSession?.let { TransactionVoidHost(it, submit) {
        selectedId = null
        onVoided?.invoke()
    } }
}

private fun transactionTitle(event: Transaction, state: TransactionBrowserState): String =
    event.description?.takeIf { it.isNotBlank() } ?: when (event.kind) {
        TransactionKind.TRANSFER -> "${event.accountId?.let { accountLabel(it, state) }} → ${event.destinationAccountId?.let { accountLabel(it, state) }}"
        else -> event.categoryId?.let { id -> state.categories.firstOrNull { it.id == id }?.name }
            ?: transactionKind(event.kind)
    }

private fun transactionAmounts(event: Transaction): String {
    if (event.kind == TransactionKind.TRANSFER) {
        val sent = "${event.amount.currency.name} ${accountAmount(event.amount)} sent"
        return event.receivedAmount?.let { "$sent → ${it.currency.name} ${accountAmount(it)} received" } ?: sent
    }
    val sign = when {
        event.amount.minorUnits < 0 || event.kind == TransactionKind.EXPENSE -> "−"
        event.amount.minorUnits > 0 -> "+"
        else -> ""
    }
    return "$sign${event.amount.currency.name} ${accountAmount(event.amount).removePrefix("-")}"
}

@Composable
private fun TransactionMetadata(event: Transaction, state: TransactionBrowserState) {
    if (event.state == TransactionState.VOIDED) Text("Voided", color = MaterialTheme.colorScheme.onSurfaceVariant)
    Text(event.accountId?.let { "Account: ${accountLabel(it, state)}" } ?: "No account assigned",
        style = MaterialTheme.typography.bodyMedium)
    event.destinationAccountId?.let { Text("Destination: ${accountLabel(it, state)}", style = MaterialTheme.typography.bodyMedium) }
    event.categoryId?.let { id ->
        Text("Category: " + (state.categories.firstOrNull { it.id == id }?.name ?: "Category unavailable"),
            style = MaterialTheme.typography.bodyMedium)
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
