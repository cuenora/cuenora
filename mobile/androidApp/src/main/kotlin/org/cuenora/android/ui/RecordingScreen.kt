package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.cuenora.shared.domain.*
import org.cuenora.shared.recording.*

@Composable
fun RecordingEntry(begin: () -> Unit) {
    Column(Modifier.fillMaxSize().padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text("Transactions", style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
        Text("Record income or an expense, with or without an account.")
        CuenoraAction("Record income or expense", begin, Modifier.testTag("record-from-transactions"))
        Text("Transaction history and filters are not available yet.")
    }
}

/** Full-size modal surface retains the originating navigation context underneath it. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RecordingScreen(flow: RecordingSession, submit: (suspend () -> Unit) -> Unit, done: () -> Unit) {
    val state by flow.state.collectAsState()
    var zone by remember { mutableStateOf(java.util.TimeZone.getDefault().id) }
    var discard by remember { mutableStateOf(false) }
    val cancellable = state.stage in setOf(RecordingStage.Form, RecordingStage.Review)
    val requestBack = {
        if (!state.busy) {
            if (cancellable) discard = true else done()
        }
    }
    BackHandler { requestBack() }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = { discard = false; done() })
    Scaffold(
        modifier = Modifier.fillMaxSize().testTag("recording-screen"),
        containerColor = MaterialTheme.colorScheme.background,
        contentWindowInsets = WindowInsets.safeDrawing,
        topBar = {
            TopAppBar(
                navigationIcon = {
                    IconButton(onClick = requestBack, enabled = !state.busy) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                title = {},
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                    scrolledContainerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding()
            .verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(when (state.stage) {
                RecordingStage.Review -> "Review your record"
                RecordingStage.Uncertain -> "Resolve original record"
                RecordingStage.Saved -> "Record saved"
                else -> "Record income or expense"
            }, style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            when (state.stage) {
                RecordingStage.Loading -> CuenoraLoading("Checking your profile and recording choices…")
                RecordingStage.Form -> RecordingFields(state, flow)
                RecordingStage.Review, RecordingStage.Uncertain -> {
                    val form = state.form
                    CuenoraAmount(if (form.kind == TransactionKind.INCOME) "Income" else "Expense", form.currency.name, form.amount)
                    Text("Business date: ${form.occurredOn}")
                    Text("Profile time zone: ${state.profile?.timeZone ?: "Unavailable"}")
                    Text("Account: ${state.accounts.firstOrNull { it.id == form.accountId }?.name?.value ?: if (form.accountId == null) "No account assigned" else "Original assigned account"}")
                    Text(if (form.accountId == null) "Included in reports; no account balance changes."
                        else if (form.kind == TransactionKind.INCOME) "Increases the selected account's recorded balance."
                        else "Decreases the selected account's recorded balance.")
                    form.description?.let { Text("Description: $it") }
                    form.categoryId?.let { id -> Text("Category: ${state.categories.firstOrNull { it.id == id }?.name ?: "Original selected category"}") }
                    Text("This records activity that already happened. It does not move money at a bank.")
                    CuenoraAction(if (state.stage == RecordingStage.Review) "Save record" else "Retry original request",
                        { submit { flow.save() } }, Modifier.testTag("record-save"), busy = state.busy)
                    if (state.stage == RecordingStage.Review) TextButton(onClick = flow::edit, enabled = !state.busy) { Text("Keep editing") }
                    else Text("The original details and operation key are preserved securely. Leaving does not discard the unresolved operation.")
                }
                RecordingStage.Saved -> {
                    CuenoraNotice("Recorded", "The server confirmed this record. Do not submit it again.", NoticeTone.Success)
                    CuenoraAction("Done", done, Modifier.testTag("record-done"))
                }
                RecordingStage.Failed -> CuenoraAction("Try again", { submit { flow.retryLoad() } }, Modifier.testTag("record-reload"))
                RecordingStage.Blocked -> {
                    if (state.profile?.emailVerified == true) {
                        Text("Explicitly confirm the profile time zone used for business dates. The device zone is only a suggestion.")
                        CuenoraField("IANA time zone", zone, { zone = it }, Modifier.testTag("record-zone"))
                        CuenoraAction("Confirm time zone", { submit { flow.confirmZone(zone) } },
                            Modifier.testTag("record-confirm-zone"), busy = state.busy)
                    } else Text("Return to Home to verify your email before recording activity.")
                }
                RecordingStage.Closed -> Unit
            }
            state.notice?.let { CuenoraNotice("Recording status", it, NoticeTone.Attention) }
            if (state.stage != RecordingStage.Saved) TextButton(onClick = { if (cancellable) discard = true else done() },
                enabled = !state.busy, modifier = Modifier.testTag("record-cancel")) {
                Text(if (cancellable) "Cancel record" else "Back")
            }
        }
    }
}

@Composable
private fun RecordingFields(state: RecordingState, flow: RecordingSession) {
    val form = state.form
    fun error(field: String, message: String) = if (state.issues.any { it.field == field }) message else null
    Text("Record type")
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        listOf(TransactionKind.INCOME, TransactionKind.EXPENSE).forEach { kind ->
            FilterChip(selected = form.kind == kind, onClick = { flow.update(form.copy(kind = kind, categoryId = null)) },
                label = { Text(if (kind == TransactionKind.INCOME) "Income" else "Expense") }, modifier = Modifier.testTag("record-${kind.name}"))
        }
    }
    CuenoraField("Amount", form.amount, { flow.update(form.copy(amount = it)) }, Modifier.testTag("record-amount"),
        error = error("amount", "Enter a positive decimal amount with at most two decimal places."))
    Text("Currency")
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Currency.entries.forEach { currency ->
            FilterChip(selected = form.currency == currency, onClick = { flow.update(form.copy(currency = currency)) }, label = { Text(currency.name) })
        }
    }
    CuenoraField("Business date (YYYY-MM-DD)", form.occurredOn, { flow.update(form.copy(occurredOn = it)) },
        Modifier.testTag("record-date"), error = error("occurredOn", "Enter a valid date no later than today in your profile time zone."))
    Text("Profile time zone: ${state.profile?.timeZone}. Enter when the activity happened, not a future plan.")
    CuenoraField("Description (optional)", form.description.orEmpty(), { flow.update(form.copy(description = it.ifEmpty { null })) },
        Modifier.testTag("record-description"), error = error("description", "Enter a valid description without unsupported control characters."))
    ChoiceMenu("Account", state.accounts.firstOrNull { it.id == form.accountId }?.name?.value ?: "No account assigned",
        listOf("No account assigned") + state.accounts.filter { it.status == AccountStatus.ACTIVE &&
            it.currency == form.currency && (form.kind != TransactionKind.INCOME || it.type == AccountType.NORMAL) }.map { it.name.value }) { index ->
        val eligible = state.accounts.filter { it.status == AccountStatus.ACTIVE && it.currency == form.currency &&
            (form.kind != TransactionKind.INCOME || it.type == AccountType.NORMAL) }
        flow.update(form.copy(accountId = if (index == 0) null else eligible[index - 1].id))
    }
    if (form.accountId == null) Text("No account balance changes; this record still contributes to income or expense reports.")
    else Text("The assigned account must be active and use the same currency. Ordinary income cannot target savings.")
    error("accountId", "Choose an eligible same-currency account, or explicitly select No account assigned.")?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    if (state.accountsUnavailable) Text("Accounts could not be loaded. You can record without an account, or cancel and retry later.")
    val applicable = state.categories.filter { it.kind == form.kind }
    ChoiceMenu("Category (optional)", applicable.firstOrNull { it.id == form.categoryId }?.name ?: "No category",
        listOf("No category") + applicable.map { it.name }) { index ->
        flow.update(form.copy(categoryId = if (index == 0) null else applicable[index - 1].id))
    }
    if (state.categoriesUnavailable) Text("Categories could not be loaded. You can record without a category, or cancel and retry later.")
    error("categoryId", "Choose a category matching the record type.")?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    if (state.issues.any { it.field in setOf("form", "kind") }) Text("Review the record's fields before continuing.", color = MaterialTheme.colorScheme.error)
    CuenoraAction("Review record", flow::review, Modifier.testTag("record-review"))
}

@Composable
private fun ChoiceMenu(label: String, selected: String, choices: List<String>, choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        OutlinedButton(onClick = { expanded = true }, modifier = Modifier.fillMaxWidth()) { Text("$label: $selected") }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            choices.forEachIndexed { index, choice -> DropdownMenuItem(text = { Text(choice) }, onClick = { expanded = false; choose(index) }) }
        }
    }
}
