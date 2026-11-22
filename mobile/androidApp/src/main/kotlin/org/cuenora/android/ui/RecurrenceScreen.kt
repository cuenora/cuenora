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
import org.cuenora.shared.planning.ExactDecimal
import org.cuenora.shared.recurrence.*

/** UI intents contain reviewed terms, never credentials or retry keys. */
sealed interface RecurrenceIntent {
    data object Refresh : RecurrenceIntent
    data object MoreRules : RecurrenceIntent
    data object MoreOccurrences : RecurrenceIntent
    data class RuleFilter(val state: RuleState?) : RecurrenceIntent
    data class OccurrenceFilter(val state: OccurrenceState?) : RecurrenceIntent
    data class Create(val terms: RecurrenceRuleDraft) : RecurrenceIntent
    data class Edit(val id: RuleId, val terms: RecurrenceRuleDraft) : RecurrenceIntent
    data class Pause(val id: RuleId) : RecurrenceIntent
    data class Resume(val id: RuleId) : RecurrenceIntent
    data class Confirm(val id: OccurrenceId, val actual: ActualPaymentDraft) : RecurrenceIntent
    data class Skip(val id: OccurrenceId) : RecurrenceIntent
    data class Postpone(val id: OccurrenceId, val dueOn: String) : RecurrenceIntent
    data object Submit : RecurrenceIntent
    data object CancelReview : RecurrenceIntent
    data object Retry : RecurrenceIntent
}

@Composable
fun RecurrenceScreen(flow: AuthenticatedRecurrenceSession, submit: (suspend () -> Unit) -> Unit,
    changed: () -> Unit, done: () -> Unit) {
    val state by flow.state.collectAsState()
    LaunchedEffect(flow) { submit { flow.load() } }
    LaunchedEffect(state.writeSucceeded) { if (state.writeSucceeded) changed() }
    RecurrencePanel(state, { intent ->
        when (intent) {
            RecurrenceIntent.Refresh -> submit { flow.refresh() }
            RecurrenceIntent.MoreRules -> submit { flow.loadMoreRules() }
            RecurrenceIntent.MoreOccurrences -> submit { flow.loadMoreOccurrences() }
            is RecurrenceIntent.RuleFilter -> { flow.setRuleFilter(intent.state); submit { flow.refresh() } }
            is RecurrenceIntent.OccurrenceFilter -> { flow.setOccurrenceFilter(intent.state); submit { flow.refresh() } }
            is RecurrenceIntent.Create -> flow.prepareCreate(intent.terms)
            is RecurrenceIntent.Edit -> flow.prepareEdit(intent.id, intent.terms)
            is RecurrenceIntent.Pause -> flow.preparePause(intent.id)
            is RecurrenceIntent.Resume -> flow.prepareResume(intent.id)
            is RecurrenceIntent.Confirm -> submit { flow.prepareConfirm(intent.id, intent.actual) }
            is RecurrenceIntent.Skip -> flow.prepareSkip(intent.id)
            is RecurrenceIntent.Postpone -> flow.preparePostpone(intent.id, intent.dueOn)
            RecurrenceIntent.Submit -> submit { flow.submitReviewed() }
            RecurrenceIntent.CancelReview -> flow.cancelReview()
            RecurrenceIntent.Retry -> submit { flow.retryPending() }
        }
    }, done)
}

private sealed interface RecurrenceEditor {
    data class Rule(val original: RecurrenceRule?) : RecurrenceEditor
    data class Payment(val occurrence: RecurrenceOccurrence) : RecurrenceEditor
    data class Postpone(val occurrence: RecurrenceOccurrence) : RecurrenceEditor
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RecurrencePanel(state: RecurrenceScreenState, action: (RecurrenceIntent) -> Unit, done: () -> Unit) {
    var managingRules by remember { mutableStateOf(false) }
    var selectedOccurrenceId by remember { mutableStateOf<OccurrenceId?>(null) }
    val selectedOccurrence = state.occurrences.firstOrNull { it.id == selectedOccurrenceId }
    var editor by remember { mutableStateOf<RecurrenceEditor?>(null) }
    var discard by remember { mutableStateOf(false) }
    var ruleDraft by remember(editor) { mutableStateOf((editor as? RecurrenceEditor.Rule)?.original?.let {
        RecurrenceRuleDraft(accountAmount(it.amount), it.amount.currency, it.cadence, recurrenceDate(it.startsOn),
            it.endsOn?.let(::recurrenceDate), it.timeZone, it.accountId, it.categoryId, it.description)
    } ?: RecurrenceRuleDraft("", Currency.SEK, "monthly", "", timeZone = "")) }
    var paymentDraft by remember(editor) { mutableStateOf((editor as? RecurrenceEditor.Payment)?.occurrence?.let(::actualPaymentDraft)
        ?: ActualPaymentDraft("", Currency.SEK, "")) }
    var postponedDate by remember(editor) { mutableStateOf("") }
    LaunchedEffect(state.writeSucceeded) {
        if (state.writeSucceeded) {
            editor = null
            selectedOccurrenceId = null
        }
    }
    val back = {
        if (!state.busy) {
            when {
                state.pendingRetry -> done()
                state.review != null -> action(RecurrenceIntent.CancelReview)
                editor != null -> discard = true
                selectedOccurrence != null -> selectedOccurrenceId = null
                managingRules -> managingRules = false
                else -> done()
            }
        }
    }
    BackHandler { back() }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = {
        discard = false; editor = null; action(RecurrenceIntent.CancelReview)
    })
    val overviewScroll = rememberScrollState()
    val secondaryView = state.pendingRetry || state.review != null || state.requiresRefresh ||
        editor != null || selectedOccurrence != null || managingRules
    val scroll = if (secondaryView) {
        key(state.pendingRetry, state.review != null, state.requiresRefresh, editor, selectedOccurrenceId, managingRules) {
            rememberScrollState()
        }
    } else overviewScroll
    Scaffold(modifier = Modifier.fillMaxSize().testTag("recurrence-screen"),
        containerColor = MaterialTheme.colorScheme.background, contentWindowInsets = WindowInsets.safeDrawing,
        topBar = { TopAppBar(title = { Text(when {
            !secondaryView -> "Recurrence"
            managingRules && editor == null && state.review == null && !state.pendingRetry -> "Recurring rules"
            selectedOccurrence != null && editor == null && state.review == null && !state.pendingRetry -> "Recurrence"
            else -> "Recurring payments"
        }) }, navigationIcon = {
            IconButton(onClick = back, enabled = !state.busy) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
        }) }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding()
            .verticalScroll(scroll).padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            if (state.busy) CuenoraLoading("Checking recurrence data…")
            state.notice?.let { notice -> CuenoraNotice("Recurrence status", when (notice) {
                RecurrenceNotice.AccessDenied -> "Return to Home to verify your email or sign in again."
                RecurrenceNotice.RequestRejected -> "The server rejected this new request without accepting it. Correct the details and review again."
                RecurrenceNotice.InvalidForm -> "Check the amount, dates, time zone, account and category before reviewing."
                RecurrenceNotice.ZoneRequired -> "Confirm your profile time zone on Home before recording an actual payment."
                RecurrenceNotice.WriteUncertain -> "The outcome is unknown. Only the original request may be retried."
                RecurrenceNotice.RefreshRequired -> "Refresh and review the current rule before another action. The earlier change may have succeeded."
                RecurrenceNotice.StorageUnavailable -> "Secure recovery is unavailable. No new operation can be safely started."
                RecurrenceNotice.NetworkUnavailable -> "Could not load current data. Check your connection and refresh."
                RecurrenceNotice.ServiceUnavailable -> "Current data could not be verified. Refresh before continuing."
            }, NoticeTone.Attention) }
            state.issues.forEach { Text("Check ${it.field}: ${it.code.replace('_', ' ')}", color = MaterialTheme.colorScheme.error) }
            when {
                state.pendingRetry -> {
                    Text("Resolve original request", style = MaterialTheme.typography.headlineSmall)
                    Text("The original details and operation key are preserved securely. Leaving does not discard the unresolved operation. Refreshing does not resubmit it.")
                    (state.pendingReview ?: state.review)?.let { RecurrenceReviewDetails(it, state) }
                    CuenoraAction("Retry original request", { action(RecurrenceIntent.Retry) }, busy = state.busy)
                }
                state.review != null -> {
                    val confirmingPayment = state.review is RecurrenceReview.Confirm
                    state.review?.let { RecurrenceReviewDetails(it, state) }
                    CuenoraAction(if (confirmingPayment) "Confirm payment" else "Apply planning change",
                        { action(RecurrenceIntent.Submit) }, Modifier.testTag("recurrence-submit"), busy = state.busy)
                    TextButton(onClick = { action(RecurrenceIntent.CancelReview) }, enabled = !state.busy) { Text("Keep editing") }
                    if (confirmingPayment) {
                        OutlinedButton(onClick = {
                            editor = null
                            action(RecurrenceIntent.CancelReview)
                        }, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) { Text("Cancel") }
                    }
                }
                state.requiresRefresh -> CuenoraAction("Refresh current state", { action(RecurrenceIntent.Refresh) }, busy = state.busy)
                editor != null -> key(editor) {
                    val current = editor
                    when (current) {
                        is RecurrenceEditor.Rule -> RuleFields(current.original, state, ruleDraft, { ruleDraft = it }, { terms ->
                            action(current.original?.let { RecurrenceIntent.Edit(it.id, terms) } ?: RecurrenceIntent.Create(terms))
                        })
                        is RecurrenceEditor.Payment -> PaymentFields(current.occurrence, state, paymentDraft, { paymentDraft = it }) { actual -> action(RecurrenceIntent.Confirm(current.occurrence.id, actual)) }
                        is RecurrenceEditor.Postpone -> PostponeFields(current.occurrence, state.busy, postponedDate, { postponedDate = it }) { date -> action(RecurrenceIntent.Postpone(current.occurrence.id, date)) }
                        null -> Unit
                    }
                    if (current is RecurrenceEditor.Rule && current.original == null) {
                        OutlinedButton(onClick = { discard = true }, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) {
                            Text("Cancel")
                        }
                    } else {
                        TextButton(onClick = { if (current is RecurrenceEditor.Payment) editor = null else discard = true }, enabled = !state.busy) {
                            Text(if (current is RecurrenceEditor.Payment) "Not paid yet" else "Cancel changes")
                        }
                    }
                }
                selectedOccurrence != null -> OccurrenceDetails(
                    selectedOccurrence,
                    state,
                    recordPayment = {
                        editor = RecurrenceEditor.Payment(selectedOccurrence)
                        action(RecurrenceIntent.Confirm(selectedOccurrence.id, actualPaymentDraft(selectedOccurrence)))
                    },
                    notPaid = { selectedOccurrenceId = null },
                    changeDueDate = { editor = RecurrenceEditor.Postpone(selectedOccurrence) },
                    skip = { action(RecurrenceIntent.Skip(selectedOccurrence.id)) },
                )
                managingRules -> {
                    TextButton(onClick = { action(RecurrenceIntent.Refresh) }, enabled = !state.busy) { Text("Refresh") }
                    if (state.writeSucceeded) CuenoraNotice("Saved", "The server confirmed the operation. Lists show current server data after refresh.", NoticeTone.Success)
                    RecurrenceChoice("Rule state", state.ruleFilter?.name?.lowercase() ?: "all", listOf("active", "paused", "all"), !state.busy) { index ->
                        action(RecurrenceIntent.RuleFilter(RuleState.entries.getOrNull(index)))
                    }
                    if (state.rules.isEmpty() && !state.busy && state.notice == null) Text("No rules in this view.")
                    state.rules.forEach { rule ->
                        HorizontalDivider()
                        Text(rule.description ?: "Recurring expense rule", style = MaterialTheme.typography.titleMedium)
                        Text("${accountAmount(rule.amount)} ${rule.amount.currency.name} · ${rule.cadence} · ${rule.state.name.lowercase()}")
                        Text("Starts: ${recurrenceDate(rule.startsOn)} · Ends: ${rule.endsOn?.let(::recurrenceDate) ?: "No end"}")
                        Text("Rule time zone: ${rule.timeZone} · Account: ${accountName(state, rule.accountId)}")
                        if (!state.busy && state.notice == null) {
                            TextButton(onClick = { editor = RecurrenceEditor.Rule(rule) }) { Text("Edit rule") }
                            TextButton(onClick = { action(if (rule.state == RuleState.ACTIVE) RecurrenceIntent.Pause(rule.id) else RecurrenceIntent.Resume(rule.id)) }) {
                                Text(if (rule.state == RuleState.ACTIVE) "Pause rule" else "Resume rule")
                            }
                        }
                    }
                    if (state.hasMoreRules) TextButton(onClick = { action(RecurrenceIntent.MoreRules) }, enabled = !state.busy) { Text("Load more rules") }
                }
                else -> {
                    if (state.writeSucceeded) CuenoraNotice("Saved", "The server confirmed the operation. Lists show current server data after refresh.", NoticeTone.Success)
                    if (state.profile?.emailVerified == true && !state.busy && state.notice == null) {
                        CuenoraAction("Add recurring expense", { editor = RecurrenceEditor.Rule(null) })
                    }
                    OutlinedButton(onClick = { managingRules = true }, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) {
                        Text("Manage recurring rules")
                    }
                    UpcomingPayments(state, action) { selectedOccurrenceId = it.id }
                }
            }
        }
    }
}

@Composable
private fun OccurrenceDetails(
    occurrence: RecurrenceOccurrence,
    state: RecurrenceScreenState,
    recordPayment: () -> Unit,
    notPaid: () -> Unit,
    changeDueDate: () -> Unit,
    skip: () -> Unit,
) {
    val planned = occurrence.state == OccurrenceState.PLANNED
    val canAct = !state.busy && state.notice == null
    val account = state.accounts.firstOrNull { it.account.id == occurrence.accountId }
    Text(occurrence.description ?: "Recurring expense", style = MaterialTheme.typography.headlineSmall,
        modifier = Modifier.semantics { heading() })
    Text(when (occurrence.state) {
        OccurrenceState.PLANNED -> "Planned · Not confirmed as paid"
        OccurrenceState.CONFIRMED -> "Confirmed"
        OccurrenceState.SKIPPED -> "Skipped"
    })
    Text("${occurrence.amount.currency.name} ${accountAmount(occurrence.amount)}",
        style = MaterialTheme.typography.headlineMedium, fontFamily = UiFont)
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Due date", style = MaterialTheme.typography.titleSmall)
        Text(recurrenceDate(occurrence.dueOn))
        if (occurrence.scheduledOn != occurrence.dueOn) {
            Text("Originally scheduled: ${recurrenceDate(occurrence.scheduledOn)}", style = MaterialTheme.typography.bodySmall)
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Source account", style = MaterialTheme.typography.titleSmall)
        Text(account?.let { "${it.account.name.value} · ${it.account.currency.name}" }
            ?: accountName(state, occurrence.accountId))
        account?.let {
            Text("Recorded balance", style = MaterialTheme.typography.labelMedium)
            Text("${it.balance.currency.name} ${accountAmount(it.balance)}", fontFamily = UiFont)
        }
    }
    if (planned) {
        Text("The due date does not pay or record this expense.")
        if (canAct) {
            CuenoraAction("Record actual payment", recordPayment)
            Text("Use this after you have paid. Cuenora does not send money.", style = MaterialTheme.typography.bodySmall)
        }
        OutlinedButton(onClick = notPaid, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) {
            Text("Not paid yet")
        }
        if (canAct) {
            HorizontalDivider()
            Text("This occurrence", style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
            TextButton(onClick = changeDueDate) { Text("Change due date") }
            TextButton(onClick = skip) { Text("Skip this occurrence") }
        }
    }
}

@Composable
private fun UpcomingPayments(
    state: RecurrenceScreenState,
    action: (RecurrenceIntent) -> Unit,
    review: (RecurrenceOccurrence) -> Unit,
) {
    val plannedOnly = state.occurrenceFilter == OccurrenceState.PLANNED
    Text(if (plannedOnly) "Upcoming payments" else "Recurring occurrences",
        style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
    Text("Planned payments do not change balances or realized expense reports.")
    RecurrenceChoice("Occurrence state", state.occurrenceFilter?.name?.lowercase() ?: "all",
        listOf("planned", "confirmed", "skipped", "all"), !state.busy) { index ->
        action(RecurrenceIntent.OccurrenceFilter(OccurrenceState.entries.getOrNull(index)))
    }
    if (state.occurrences.isEmpty() && !state.busy && state.notice == null) {
        Text(if (plannedOnly) "No upcoming payments." else "No occurrences in this view.")
        Text("Creating a rule does not immediately generate or post a payment.")
    }
    state.occurrences.groupBy { it.dueOn }.forEach { (dueOn, occurrences) ->
        Text(recurrenceDate(dueOn), style = MaterialTheme.typography.titleSmall, modifier = Modifier.semantics { heading() })
        occurrences.forEach { occurrence ->
            val title = occurrence.description ?: "Recurring expense"
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                HorizontalDivider()
                Text(title, style = MaterialTheme.typography.titleMedium)
                Text("${accountAmount(occurrence.amount)} ${occurrence.amount.currency.name}")
                Text(occurrenceContext(occurrence, state))
                if (occurrence.state == OccurrenceState.PLANNED) {
                    TextButton(onClick = { review(occurrence) }, enabled = !state.busy && state.notice == null) {
                        Text("Review $title")
                    }
                } else {
                    Text("Scheduled: ${recurrenceDate(occurrence.scheduledOn)}")
                }
            }
        }
    }
    if (state.hasMoreOccurrences) TextButton(onClick = { action(RecurrenceIntent.MoreOccurrences) }, enabled = !state.busy) { Text("Load more occurrences") }
    TextButton(onClick = { action(RecurrenceIntent.Refresh) }, enabled = !state.busy) { Text("Refresh") }
}

private fun occurrenceContext(occurrence: RecurrenceOccurrence, state: RecurrenceScreenState) =
    "${occurrence.state.name.lowercase().replaceFirstChar { it.uppercase() }} · ${accountName(state, occurrence.accountId)}"

@Composable
private fun RuleFields(original: RecurrenceRule?, state: RecurrenceScreenState, draft: RecurrenceRuleDraft,
    update: (RecurrenceRuleDraft) -> Unit, review: (RecurrenceRuleDraft) -> Unit) {
    val creating = original == null
    Text(if (creating) "Add recurring expense" else "Edit recurring rule", style = MaterialTheme.typography.headlineSmall,
        modifier = if (creating) Modifier.semantics { heading() } else Modifier)
    if (creating) RuleDescription(draft, update)
    CuenoraField(if (creating) "Amount" else "Planned amount", draft.amount, { update(draft.copy(amount = it)) },
        Modifier.testTag("recurrence-rule-amount"), decimal = true, canonicalizeDecimal = true, enabled = !state.busy)
    RecurrenceChoice("Currency", draft.currency.name, Currency.entries.map { it.name }, !state.busy) { update(draft.copy(currency = Currency.entries[it], accountId = null)) }
    if (creating) {
        RecurrenceCategoryChoice(state, draft.categoryId, "Category") { update(draft.copy(categoryId = it)) }
        RecurrenceAccountChoice(state, draft.currency, draft.accountId, "Source account") { update(draft.copy(accountId = it)) }
        Text("You can also leave the account unassigned.", style = MaterialTheme.typography.bodySmall)
    }
    val cadences = listOf("weekly", "monthly", "quarterly", "annual")
    RecurrenceChoice(if (creating) "Repeat" else "Cadence", draft.cadence, cadences, !state.busy) {
        update(draft.copy(cadence = cadences[it]))
    }
    CuenoraDateField(if (creating) "First due date" else "Start date", draft.startsOn, { update(draft.copy(startsOn = it)) },
        Modifier.testTag("recurrence-start"), enabled = !state.busy)
    if (!creating) RuleEndDate(draft, update, state.busy)
    CuenoraField(if (creating) "Schedule time zone" else "Explicit IANA rule time zone", draft.timeZone,
        { update(draft.copy(timeZone = it)) }, Modifier.testTag("recurrence-zone"))
    Text("The rule keeps its own time zone. Month-end adjustments preserve the original calendar anchor.")
    if (creating) {
        RuleEndDate(draft, update, state.busy)
        Text("Saving a schedule creates planned payments. Record each actual payment only after you have paid it.")
    } else {
        RecurrenceReferences(state, draft.currency, draft.accountId, draft.categoryId,
            { update(draft.copy(accountId = it)) }, { update(draft.copy(categoryId = it)) })
        RuleDescription(draft, update)
        Text("Edits apply only to new occurrences. Existing planned, skipped and confirmed items are preserved.")
    }
    CuenoraAction(if (creating) "Review schedule" else "Review rule", { review(draft) }, busy = state.busy)
}

@Composable
private fun RuleEndDate(draft: RecurrenceRuleDraft, update: (RecurrenceRuleDraft) -> Unit, busy: Boolean) {
    CuenoraDateField("End date (optional, inclusive)", draft.endsOn.orEmpty(),
        { update(draft.copy(endsOn = it.ifEmpty { null })) }, Modifier.testTag("recurrence-end"),
        enabled = !busy, optional = true)
}

@Composable
private fun RuleDescription(draft: RecurrenceRuleDraft, update: (RecurrenceRuleDraft) -> Unit) {
    CuenoraField("Description (optional)", draft.description.orEmpty(),
        { update(draft.copy(description = it.ifEmpty { null })) }, Modifier.testTag("recurrence-description"))
}

@Composable
private fun PaymentFields(occurrence: RecurrenceOccurrence, state: RecurrenceScreenState, draft: ActualPaymentDraft,
    update: (ActualPaymentDraft) -> Unit, review: (ActualPaymentDraft) -> Unit) {
    Text("Actual payment details", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
    Text("Confirm only after you have paid this expense.")
    Text("Planned suggestion: ${accountAmount(occurrence.amount)} ${occurrence.amount.currency.name}, due ${recurrenceDate(occurrence.dueOn)}. Enter what actually happened.")
    CuenoraField("Actual amount paid", draft.amount, { update(draft.copy(amount = it)) },
        Modifier.testTag("recurrence-actual-amount"), decimal = true, canonicalizeDecimal = true, enabled = !state.busy)
    RecurrenceChoice("Actual currency", draft.currency.name, Currency.entries.map { it.name }, !state.busy) { update(draft.copy(currency = Currency.entries[it], accountId = null)) }
    CuenoraDateField("Payment date", draft.occurredOn, { update(draft.copy(occurredOn = it)) },
        Modifier.testTag("recurrence-actual-date"), enabled = !state.busy)
    Text("The business date of the actual payment.", style = MaterialTheme.typography.bodySmall)
    Text("Profile time zone: ${state.profile?.timeZone ?: "Not confirmed; return to Home"}")
    RecurrenceReferences(state, draft.currency, draft.accountId, draft.categoryId,
        { update(draft.copy(accountId = it)) }, { update(draft.copy(categoryId = it)) }, accountLabel = "Paid from")
    CuenoraField("Actual description (optional)", draft.description.orEmpty(), { update(draft.copy(description = it.ifEmpty { null })) }, Modifier.testTag("recurrence-actual-description"))
    Text("This records a payment already made. It does not instruct a bank to pay.")
    CuenoraAction("Review actual terms", { review(draft) }, busy = state.busy)
}

@Composable
private fun PostponeFields(occurrence: RecurrenceOccurrence, busy: Boolean, date: String, update: (String) -> Unit, review: (String) -> Unit) {
    Text("Postpone one occurrence", style = MaterialTheme.typography.headlineSmall)
    Text("Current due date: ${recurrenceDate(occurrence.dueOn)}. Choose a strictly later date; the scheduled identity and rule calendar stay unchanged.")
    CuenoraDateField("New due date", date, update, Modifier.testTag("recurrence-postpone-date"), enabled = !busy)
    CuenoraAction("Review postponement", { review(date) }, busy = busy)
}

@Composable
private fun RecurrenceReferences(state: RecurrenceScreenState, currency: Currency, account: AccountId?, category: CategoryId?,
    selectAccount: (AccountId?) -> Unit, selectCategory: (CategoryId?) -> Unit, accountLabel: String = "Account") {
    RecurrenceAccountChoice(state, currency, account, accountLabel, selectAccount)
    RecurrenceCategoryChoice(state, category, "Expense category", selectCategory)
}

@Composable
private fun RecurrenceAccountChoice(state: RecurrenceScreenState, currency: Currency, account: AccountId?,
    label: String, selectAccount: (AccountId?) -> Unit) {
    val eligible = state.accounts.map { it.account }.filter { it.status == AccountStatus.ACTIVE && it.currency == currency }
    RecurrenceChoice(label, accountName(state, account), listOf("No account assigned") + eligible.map { it.name.value }, !state.busy) {
        selectAccount(if (it == 0) null else eligible[it - 1].id)
    }
}

@Composable
private fun RecurrenceCategoryChoice(state: RecurrenceScreenState, category: CategoryId?, label: String,
    selectCategory: (CategoryId?) -> Unit) {
    val categories = state.categories.filter { it.kind == TransactionKind.EXPENSE }
    RecurrenceChoice(label, categories.firstOrNull { it.id == category }?.name ?: "No category",
        listOf("No category") + categories.map { it.name }, !state.busy) { selectCategory(if (it == 0) null else categories[it - 1].id) }
}

@Composable
private fun RecurrenceChoice(label: String, selected: String, options: List<String>, enabled: Boolean, choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        OutlinedButton(onClick = { expanded = true }, enabled = enabled, modifier = Modifier.fillMaxWidth()) { Text("$label: $selected") }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEachIndexed { index, text -> DropdownMenuItem(text = { Text(text) }, onClick = { expanded = false; choose(index) }) }
        }
    }
}

@Composable
private fun RecurrenceReviewDetails(review: RecurrenceReview, state: RecurrenceScreenState) {
    val confirmingPayment = review is RecurrenceReview.Confirm && !state.pendingRetry
    Text(if (confirmingPayment) "Confirm payment" else "Review before applying",
        style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
    if (confirmingPayment) Text("Confirm only after you have paid this expense.")
    val ruleId = when (review) {
        is RecurrenceReview.EditRule -> review.id
        is RecurrenceReview.PauseRule -> review.id
        is RecurrenceReview.ResumeRule -> review.id
        else -> null
    }
    val occurrenceId = when (review) {
        is RecurrenceReview.Confirm -> review.id
        is RecurrenceReview.Skip -> review.id
        is RecurrenceReview.Postpone -> review.id
        else -> null
    }
    ruleId?.let { id ->
        val rule = state.rules.firstOrNull { it.id == id }
        Text("Rule: ${rule?.description ?: java.util.UUID(id.highBits, id.lowBits).toString()}")
        rule?.let { Text("Current plan: ${accountAmount(it.amount)} ${it.amount.currency.name}, ${it.cadence}, ${it.timeZone}") }
    }
    occurrenceId?.let { id ->
        val occurrence = state.occurrences.firstOrNull { it.id == id }
        Text("Occurrence: ${occurrence?.description ?: java.util.UUID(id.highBits, id.lowBits).toString()}")
        occurrence?.let { Text("Scheduled: ${recurrenceDate(it.scheduledOn)} · Current due: ${recurrenceDate(it.dueOn)}") }
    }
    when (review) {
        is RecurrenceReview.CreateRule -> RuleReviewTerms(review.terms, state)
        is RecurrenceReview.EditRule -> { RuleReviewTerms(review.terms, state); Text("Changes apply to future generation, never existing occurrences. Schedule or zone changes also advance generation coverage past the edit day.") }
        is RecurrenceReview.PauseRule -> { Text("Pause new generation for this rule."); Text("Existing occurrences remain unchanged and actionable.") }
        is RecurrenceReview.ResumeRule -> { Text("Resume new generation on the original calendar."); Text("Paused dates are not backfilled. Existing occurrences remain unchanged and actionable.") }
        is RecurrenceReview.Confirm -> {
            Text("Actual amount: ${review.actual.amount} ${review.actual.currency.name}")
            Text("Actual payment date: ${review.actual.occurredOn}")
            Text("Account: ${accountName(state, review.actual.accountId)}")
            Text("Category: ${state.categories.firstOrNull { it.id == review.actual.categoryId }?.name ?: "No category"}")
            Text("Description: ${review.actual.description ?: "None"}")
            if (confirmingPayment) {
                PaymentEffectPreview(review.actual, state)
            } else {
                Text(if (review.actual.accountId == null) "Included in expense reports; no account balance changes." else "Creates one expense and decreases the selected account's recorded balance.")
                Text("Only this occurrence becomes confirmed. This does not execute a bank payment.")
            }
        }
        is RecurrenceReview.Skip -> Text("Skip only this occurrence. No expense or balance movement is created. This cannot be reopened.")
        is RecurrenceReview.Postpone -> Text("Move only this occurrence's due date to ${review.dueOn}. No expense or balance movement is created; its original scheduled identity and rule calendar stay unchanged.")
    }
}

@Composable
private fun PaymentEffectPreview(actual: ActualPaymentDraft, state: RecurrenceScreenState) {
    Text("Review the recorded effect", style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
    val account = state.accounts.firstOrNull {
        it.account.id == actual.accountId && it.account.status == AccountStatus.ACTIVE && it.balance.currency == actual.currency
    }
    when {
        actual.accountId == null -> Text("Included in expense reports; no account balance changes.")
        account == null -> Text("Creates one expense. A balance estimate is not available for the selected account.")
        else -> {
            val before = accountAmount(account.balance)
            val after = ExactDecimal.subtract(before, actual.amount)
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Recorded balance before", style = MaterialTheme.typography.labelMedium)
                Text("${actual.currency.name} $before", fontFamily = UiFont)
                Text("Estimated balance after recording", style = MaterialTheme.typography.labelMedium)
                Text("${actual.currency.name} $after", fontFamily = UiFont)
            }
        }
    }
    Text("This occurrence will leave pending payments. Other occurrences stay unchanged.")
    Text("This does not execute a bank payment.", style = MaterialTheme.typography.bodySmall)
}

@Composable
private fun RuleReviewTerms(terms: RecurrenceRuleDraft, state: RecurrenceScreenState) {
    Text("Planned amount: ${terms.amount} ${terms.currency.name}")
    Text("Cadence: ${terms.cadence} · Start: ${terms.startsOn} · End: ${terms.endsOn ?: "No end"}")
    Text("Rule time zone: ${terms.timeZone}")
    Text("Account: ${accountName(state, terms.accountId)}")
    Text("Category: ${state.categories.firstOrNull { it.id == terms.categoryId }?.name ?: "No category"}")
    Text("Description: ${terms.description ?: "None"}")
    Text("Planning only: no expense or balance movement is created.")
}
private fun recurrenceDate(date: BusinessDate) = "%04d-%02d-%02d".format(java.util.Locale.ROOT, date.year, date.month, date.day)
private fun actualPaymentDraft(occurrence: RecurrenceOccurrence) = ActualPaymentDraft(
    accountAmount(occurrence.amount), occurrence.amount.currency, recurrenceDate(occurrence.dueOn),
    occurrence.accountId, occurrence.categoryId, occurrence.description,
)
private fun accountName(state: RecurrenceScreenState, id: AccountId?) = if (id == null) "No account assigned"
    else state.accounts.firstOrNull { it.account.id == id }?.account?.name?.value ?: "Assigned account (name unavailable)"
