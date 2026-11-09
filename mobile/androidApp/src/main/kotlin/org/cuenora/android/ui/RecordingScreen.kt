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
        Text("Record activity with an optional account, or a transfer between your accounts.")
        CuenoraAction("Record activity", begin, Modifier.testTag("record-from-transactions"))
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
                else -> "Record activity"
            }, style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            when (state.stage) {
                RecordingStage.Loading -> CuenoraLoading("Checking your profile and recording choices…")
                RecordingStage.Form -> RecordingFields(state, flow)
                RecordingStage.Review, RecordingStage.Uncertain -> {
                    val form = state.form
                    if (form.kind == TransactionKind.TRANSFER) {
                        TransferReview(state)
                        Text("Business date: ${form.occurredOn}")
                        Text("Profile time zone: ${state.profile?.timeZone ?: "Unavailable"}")
                    } else {
                        CuenoraAmount(if (form.kind == TransactionKind.INCOME) "Income" else "Expense", form.currency.name, form.amount)
                        Text("Business date: ${form.occurredOn}")
                        Text("Profile time zone: ${state.profile?.timeZone ?: "Unavailable"}")
                        Text("Account: ${state.accounts.firstOrNull { it.id == form.accountId }?.name?.value ?: if (form.accountId == null) "No account assigned" else "Original assigned account"}")
                        Text(if (form.accountId == null) "Included in reports; no account balance changes."
                            else if (form.kind == TransactionKind.INCOME) "Increases the selected account's recorded balance."
                            else "Decreases the selected account's recorded balance.")
                    }
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
        listOf(TransactionKind.INCOME, TransactionKind.EXPENSE, TransactionKind.TRANSFER).forEach { kind ->
            FilterChip(selected = form.kind == kind, onClick = { if (kind != form.kind) flow.update(form.copy(kind = kind, categoryId = null,
                    destinationAccountId = null, receivedAmount = null, receivedCurrency = null, quotedRate = null)) },
                label = { Text(when (kind) { TransactionKind.INCOME -> "Income"; TransactionKind.TRANSFER -> "Transfer"; else -> "Expense" }) }, modifier = Modifier.testTag("record-${kind.name}"))
        }
    }
    CuenoraField(if (form.kind == TransactionKind.TRANSFER) "Amount sent" else "Amount", form.amount, { flow.update(form.copy(amount = it)) }, Modifier.testTag("record-amount"),
        error = error("amount", "Enter a positive decimal amount with at most two decimal places."))
    if (form.kind != TransactionKind.TRANSFER) {
        Text("Currency")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Currency.entries.forEach { currency ->
                FilterChip(selected = form.currency == currency, onClick = { flow.update(form.copy(currency = currency)) }, label = { Text(currency.name) })
            }
        }
    } else Text("Sent currency: ${form.currency.name} (from source account)")
    CuenoraField("Business date (YYYY-MM-DD)", form.occurredOn, { flow.update(form.copy(occurredOn = it)) },
        Modifier.testTag("record-date"), error = error("occurredOn", "Enter a valid date no later than today in your profile time zone."))
    Text("Profile time zone: ${state.profile?.timeZone}. Enter when the activity happened, not a future plan.")
    CuenoraField("Description (optional)", form.description.orEmpty(), { flow.update(form.copy(description = it.ifEmpty { null })) },
        Modifier.testTag("record-description"), error = error("description", "Enter a valid description without unsupported control characters."))
    if (form.kind == TransactionKind.TRANSFER) TransferFields(state, flow)
    else {
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
    }
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

@Composable
private fun TransferFields(state: RecordingState, flow: RecordingSession) {
    val form = state.form
    val active = state.accounts.filter { it.status == AccountStatus.ACTIVE }
    fun label(id: AccountId?) = state.accounts.firstOrNull { it.id == id }?.let { "${it.name.value} (${it.currency.name})" } ?: "Choose account"
    ChoiceMenu("From account", label(form.accountId), active.map { "${it.name.value} (${it.currency.name})" }) { index ->
        val selected = active[index]
        flow.update(form.copy(accountId = selected.id, currency = selected.currency))
    }
    val destinations = active.filter { it.id != form.accountId }
    ChoiceMenu("To account", label(form.destinationAccountId), destinations.map { "${it.name.value} (${it.currency.name})" }) { index ->
        val selected = destinations[index]
        flow.update(form.copy(destinationAccountId = selected.id, receivedCurrency = selected.currency))
    }
    Text("Normal and savings accounts are supported. Choose two distinct active accounts. Negative resulting balances are allowed.")
    for (field in listOf("accountId", "destinationAccountId")) {
        if (state.issues.any { it.field == field }) Text("Choose a distinct active account with the matching currency.", color = MaterialTheme.colorScheme.error)
    }
    if (state.accountsUnavailable) Text("Accounts could not be loaded. Cancel and retry before recording a transfer.")
    else if (active.size < 2) Text("A transfer needs at least two active accounts. Create another account first.")
    CuenoraField("Amount received", form.receivedAmount.orEmpty(), { flow.update(form.copy(receivedAmount = it)) },
        Modifier.testTag("record-received-amount"), error = if (state.issues.any { it.field == "receivedAmount" })
            "Enter a positive amount with at most two decimal places; same-currency amounts must match." else null)
    Text("Received currency: ${form.receivedCurrency?.name ?: "Choose destination account"}. Enter the actual amount credited; it is never replaced by a quote.")
    CuenoraField("Quoted rate (optional)", form.quotedRate.orEmpty(), { flow.update(form.copy(quotedRate = it.ifEmpty { null })) },
        Modifier.testTag("record-quoted-rate"), error = if (state.issues.any { it.field == "quotedRate" })
            "Enter a positive decimal rate with at most 12 decimal places and an indicative amount within the supported range." else null)
    Text("Rate direction: ${form.receivedCurrency?.name ?: "destination currency"} per ${form.currency.name}.")
}

@Composable
private fun TransferReview(state: RecordingState) {
    val form = state.form
    fun account(id: AccountId?) = state.accounts.firstOrNull { it.id == id }?.name?.value ?: "Original account (name unavailable)"
    val receivedCurrency = form.receivedCurrency!!
    Text("From: ${account(form.accountId)} (${form.currency.name})")
    CuenoraAmount("Amount sent", form.currency.name, form.amount)
    Text("To: ${account(form.destinationAccountId)} (${receivedCurrency.name})")
    CuenoraAmount("Amount received", receivedCurrency.name, form.receivedAmount!!)
    Text("Both recorded account balances change together. This transfer does not count as income or spending.")
    form.quotedRate?.let { quote ->
        Text("Recorded quote: $quote ${receivedCurrency.name} per ${form.currency.name}")
        state.rateComparison?.let { comparison ->
            val difference = (if (comparison.difference.minorUnits > 0) "+" else "") + accountAmount(comparison.difference)
            val explanation = "The recorded rate implies ${accountAmount(comparison.indicativeReceivedAmount)} ${receivedCurrency.name}; " +
                "the confirmed received amount is ${form.receivedAmount} ${receivedCurrency.name} (difference: $difference ${receivedCurrency.name}). The confirmed amounts will be recorded."
            if (comparison.mismatch) CuenoraNotice("Rate discrepancy", explanation, NoticeTone.Attention)
            else Text(explanation)
        }
    }
}
