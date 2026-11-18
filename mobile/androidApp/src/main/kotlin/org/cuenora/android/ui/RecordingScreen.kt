package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.text.font.FontWeight
import org.cuenora.shared.domain.*
import org.cuenora.shared.recording.*
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import org.cuenora.shared.serialization.MoneyAmountSerializer

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
    val expenseForm = state.stage == RecordingStage.Form && state.form.kind == TransactionKind.EXPENSE
    val transferForm = state.stage == RecordingStage.Form && state.form.kind == TransactionKind.TRANSFER
    val incomeForm = state.stage == RecordingStage.Form && state.form.kind == TransactionKind.INCOME
    val structuredForm = expenseForm || transferForm || incomeForm
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
                    if (structuredForm) {
                        TextButton(onClick = requestBack, enabled = !state.busy,
                            modifier = Modifier.padding(start = 16.dp).heightIn(min = 48.dp).cuenoraFocus()) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back", modifier = Modifier.size(20.dp))
                            Spacer(Modifier.width(8.dp))
                            Text("Transactions")
                        }
                    } else IconButton(onClick = requestBack, enabled = !state.busy) {
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
                else -> when {
                    expenseForm -> "Record expense"
                    transferForm -> "Record transfer"
                    incomeForm -> "Record income"
                    else -> "Record activity"
                }
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
            if (structuredForm) OutlinedButton(onClick = requestBack,
                enabled = !state.busy, modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp)
                    .cuenoraFocus().testTag("record-cancel")) { Text("Cancel") }
            else if (state.stage != RecordingStage.Saved) TextButton(onClick = { if (cancellable) discard = true else done() },
                enabled = !state.busy, modifier = Modifier.testTag("record-cancel")) {
                Text(if (cancellable) "Cancel record" else "Back")
            }
        }
    }
}

@Composable
private fun RecordingFields(state: RecordingState, flow: RecordingSession) {
    if (state.form.kind == TransactionKind.TRANSFER) TransferFields(state, flow)
    else IncomeExpenseFields(state, flow)
}

/** Reuse the approved UZrhR structure while preserving each record kind's account policy. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun IncomeExpenseFields(state: RecordingState, flow: RecordingSession) {
    val form = state.form
    val income = form.kind == TransactionKind.INCOME
    val tagPrefix = if (income) "income" else "expense"
    fun error(field: String, message: String) = if (state.issues.any { it.field == field }) message else null
    Column(verticalArrangement = Arrangement.spacedBy(24.dp)) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Record type", style = MaterialTheme.typography.labelLarge)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(TransactionKind.INCOME, TransactionKind.EXPENSE, TransactionKind.TRANSFER).forEach { kind ->
                    FilterChip(selected = form.kind == kind, onClick = {
                        if (kind != form.kind) flow.update(form.copy(kind = kind, categoryId = null,
                            destinationAccountId = null, receivedAmount = null, receivedCurrency = null, quotedRate = null))
                    }, label = { Text(when (kind) {
                        TransactionKind.INCOME -> "Income"; TransactionKind.TRANSFER -> "Transfer"; else -> "Expense"
                    }) }, modifier = Modifier.testTag("record-${kind.name}"))
                }
            }
        }
        CuenoraField("Amount", form.amount, { flow.update(form.copy(amount = it)) },
            Modifier.testTag("record-amount"), decimal = true, canonicalizeDecimal = true,
            error = error("amount", "Enter a positive decimal amount with at most two decimal places."))
        RecordingChoice("Currency", form.currency.name, Currency.entries.map { it.name }, "$tagPrefix-currency") {
            flow.update(form.copy(currency = Currency.entries[it]))
        }
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CuenoraDateField("Business date", form.occurredOn, { flow.update(form.copy(occurredOn = it)) },
                Modifier.testTag("record-date"), enabled = !state.busy,
                error = error("occurredOn", "Enter a valid date no later than today in your profile time zone."))
            Text(if (income) "When the income was received. Profile time zone: ${state.profile?.timeZone}."
                else "When the expense happened. Profile time zone: ${state.profile?.timeZone}.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        CuenoraField("Description (optional)", form.description.orEmpty(),
            { flow.update(form.copy(description = it.ifEmpty { null })) }, Modifier.testTag("record-description"),
            error = error("description", "Enter a valid description without unsupported control characters."))
        val categories = state.categories.filter { it.kind == form.kind }
        RecordingChoice("Category (optional)", categories.firstOrNull { it.id == form.categoryId }?.name ?: "No category",
            listOf("No category") + categories.map { it.name }, "$tagPrefix-category") {
            flow.update(form.copy(categoryId = if (it == 0) null else categories[it - 1].id))
        }
        if (state.categoriesUnavailable) Text("Categories could not be loaded. You can record without a category, or cancel and retry later.")
        error("categoryId", "Choose a category matching the record type.")?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        val accounts = state.accounts.filter { it.status == AccountStatus.ACTIVE && it.currency == form.currency &&
            (!income || it.type == AccountType.NORMAL) }
        RecordingChoice("Account", state.accounts.firstOrNull { it.id == form.accountId }?.name?.value ?: "No account assigned",
            listOf("No account assigned") + accounts.map { it.name.value }, "$tagPrefix-account") {
            flow.update(form.copy(accountId = if (it == 0) null else accounts[it - 1].id))
        }
        error("accountId", "Choose an eligible same-currency account, or explicitly select No account assigned.")?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        if (state.accountsUnavailable) Text("Accounts could not be loaded. You can record without an account, or cancel and retry later.")
        Surface(color = MaterialTheme.colorScheme.surfaceVariant, shape = MaterialTheme.shapes.medium) {
            Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(if (form.accountId != null) "Assigned account"
                    else if (income) "Included in income reports" else "Included in spending reports",
                    style = MaterialTheme.typography.titleMedium)
                Text(if (form.accountId == null) "No account balance will change."
                    else if (income) "The assigned account must be active, normal and use the same currency. Saving increases its recorded balance."
                    else "The assigned account must be active and use the same currency. Saving decreases its recorded balance.",
                    style = MaterialTheme.typography.bodyMedium)
            }
        }
        if (state.issues.any { it.field in setOf("form", "kind") }) Text("Review the record's fields before continuing.", color = MaterialTheme.colorScheme.error)
        CuenoraAction(if (income) "Review income" else "Review expense", flow::review, Modifier.testTag("record-review"))
    }
}

@Composable
private fun RecordingChoice(label: String, selected: String, choices: List<String>, tag: String, choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().testTag(tag), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Box {
            OutlinedButton(onClick = { expanded = true }, shape = MaterialTheme.shapes.small,
                modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).cuenoraFocus()
                    .semantics { contentDescription = "$label: $selected" }) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text(selected, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                    Spacer(Modifier.width(8.dp))
                    Icon(Icons.Default.ArrowDropDown, contentDescription = null, modifier = Modifier.size(20.dp))
                }
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                choices.forEachIndexed { index, choice ->
                    DropdownMenuItem(text = { Text(choice) }, onClick = { expanded = false; choose(index) })
                }
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun TransferFields(state: RecordingState, flow: RecordingSession) {
    val form = state.form
    val active = state.accounts.filter { it.status == AccountStatus.ACTIVE }
    fun label(id: AccountId?) = state.accounts.firstOrNull { it.id == id }?.let { "${it.name.value} (${it.currency.name})" } ?: "Choose account"
    fun error(field: String, message: String) = if (state.issues.any { it.field == field }) message else null
    Column(verticalArrangement = Arrangement.spacedBy(24.dp)) {
        Text("Record a transfer you made outside Cuenora.", style = MaterialTheme.typography.bodyMedium)
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Record type", style = MaterialTheme.typography.labelLarge)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(TransactionKind.INCOME, TransactionKind.EXPENSE, TransactionKind.TRANSFER).forEach { kind ->
                    FilterChip(selected = form.kind == kind, onClick = {
                        if (kind != form.kind) flow.update(form.copy(kind = kind, categoryId = null,
                            destinationAccountId = null, receivedAmount = null, receivedCurrency = null, quotedRate = null))
                    }, label = { Text(when (kind) {
                        TransactionKind.INCOME -> "Income"; TransactionKind.TRANSFER -> "Transfer"; else -> "Expense"
                    }) }, modifier = Modifier.testTag("record-${kind.name}"))
                }
            }
        }
        RecordingChoice("From account", label(form.accountId), active.map { "${it.name.value} (${it.currency.name})" }, "transfer-from-account") { index ->
            val selected = active[index]
            flow.update(form.copy(accountId = selected.id, currency = selected.currency))
        }
        val destinations = active.filter { it.id != form.accountId }
        RecordingChoice("To account", label(form.destinationAccountId), destinations.map { "${it.name.value} (${it.currency.name})" }, "transfer-to-account") { index ->
            val selected = destinations[index]
            flow.update(form.copy(destinationAccountId = selected.id, receivedCurrency = selected.currency))
        }
        Text("Normal and savings accounts are supported. Choose two distinct active accounts. Negative resulting balances are allowed.",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        for (field in listOf("accountId", "destinationAccountId")) {
            error(field, "Choose a distinct active account with the matching currency.")?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
        if (state.accountsUnavailable) Text("Accounts could not be loaded. Cancel and retry before recording a transfer.")
        else if (active.size < 2) Text("A transfer needs at least two active accounts. Create another account first.")
        CuenoraField("Amount sent (${form.currency.name})", form.amount, { flow.update(form.copy(amount = it)) },
            Modifier.testTag("record-amount"), decimal = true, canonicalizeDecimal = true,
            error = error("amount", "Enter a positive decimal amount with at most two decimal places."))
        CuenoraField("Amount received${form.receivedCurrency?.let { " (${it.name})" }.orEmpty()}", form.receivedAmount.orEmpty(),
            { flow.update(form.copy(receivedAmount = it)) }, Modifier.testTag("record-received-amount"), decimal = true, canonicalizeDecimal = true,
            error = error("receivedAmount", "Enter a positive amount with at most two decimal places; same-currency amounts must match."))
        if (form.receivedCurrency == null) Text("Choose the destination account to see its currency.")
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CuenoraField("Reference rate (optional)", form.quotedRate.orEmpty(), { flow.update(form.copy(quotedRate = it.ifEmpty { null })) },
                Modifier.testTag("record-quoted-rate"), decimal = true, canonicalizeDecimal = true,
                error = error("quotedRate", "Enter a positive decimal rate with at most 12 decimal places and an indicative amount within the supported range."))
            Text("${form.receivedCurrency?.name ?: "Destination currency"} received per 1 ${form.currency.name} sent. No rate is required when both amounts are entered.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CuenoraDateField("Business date", form.occurredOn, { flow.update(form.copy(occurredOn = it)) },
                Modifier.testTag("record-date"), enabled = !state.busy,
                error = error("occurredOn", "Enter a valid date no later than today in your profile time zone."))
            Text("When the transfer happened. Profile time zone: ${state.profile?.timeZone}.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        CuenoraField("Description (optional)", form.description.orEmpty(), { flow.update(form.copy(description = it.ifEmpty { null })) },
            Modifier.testTag("record-description"), error = error("description", "Enter a valid description without unsupported control characters."))
        TransferMovementPreview(state)
        if (state.issues.any { it.field in setOf("form", "kind") }) Text("Review the record's fields before continuing.", color = MaterialTheme.colorScheme.error)
        CuenoraAction("Review transfer", flow::review, Modifier.testTag("record-review"))
    }
}

/** Preview entered actuals only; reference rates never supply or replace either amount. */
@Composable
private fun TransferMovementPreview(state: RecordingState) {
    val form = state.form
    val source = state.accounts.firstOrNull { it.id == form.accountId && it.status == AccountStatus.ACTIVE && it.currency == form.currency }
    val destination = state.accounts.firstOrNull { it.id == form.destinationAccountId && it.id != form.accountId &&
        it.status == AccountStatus.ACTIVE && it.currency == form.receivedCurrency }
    val sent = source?.let { transferPreviewAmount(form.amount, it.currency) }
    val received = destination?.let { transferPreviewAmount(form.receivedAmount.orEmpty(), it.currency) }
    Surface(modifier = Modifier.fillMaxWidth().testTag("transfer-movements"),
        color = MaterialTheme.colorScheme.surfaceVariant, shape = MaterialTheme.shapes.medium) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Two account movements", style = MaterialTheme.typography.titleMedium)
            if (sent != null) {
                Text(source.name.value, style = MaterialTheme.typography.bodyMedium)
                Text("−${sent.currency.name} ${accountAmount(sent)}", fontFamily = UiFont,
                    fontWeight = FontWeight.Medium, fontSize = 24.sp, lineHeight = 36.sp)
            } else Text("Choose a source account and enter a positive sent amount.")
            if (received != null) {
                Text(destination.name.value, style = MaterialTheme.typography.bodyMedium)
                Text("+${received.currency.name} ${accountAmount(received)}", fontFamily = UiFont,
                    fontWeight = FontWeight.Medium, fontSize = 24.sp, lineHeight = 36.sp)
            } else Text("Choose a destination account and enter a positive received amount.")
            Text("One transfer. No income or spending.", style = MaterialTheme.typography.bodyMedium)
            Text("Preview only. Nothing is recorded until you review and save.", style = MaterialTheme.typography.bodySmall)
        }
    }
}

private fun transferPreviewAmount(raw: String, currency: Currency): Money? = try {
    Json.decodeFromJsonElement(MoneyAmountSerializer(currency), JsonPrimitive(raw)).takeIf { it.minorUnits > 0 }
} catch (_: SerializationException) {
    null
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
