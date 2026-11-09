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
    var editor by remember { mutableStateOf<RecurrenceEditor?>(null) }
    var discard by remember { mutableStateOf(false) }
    var ruleDraft by remember(editor) { mutableStateOf((editor as? RecurrenceEditor.Rule)?.original?.let {
        RecurrenceRuleDraft(accountAmount(it.amount), it.amount.currency, it.cadence, recurrenceDate(it.startsOn),
            it.endsOn?.let(::recurrenceDate), it.timeZone, it.accountId, it.categoryId, it.description)
    } ?: RecurrenceRuleDraft("", Currency.SEK, "monthly", "", timeZone = "")) }
    var paymentDraft by remember(editor) { mutableStateOf(ActualPaymentDraft("", (editor as? RecurrenceEditor.Payment)?.occurrence?.amount?.currency ?: Currency.SEK, "")) }
    var postponedDate by remember(editor) { mutableStateOf("") }
    LaunchedEffect(state.writeSucceeded) { if (state.writeSucceeded) editor = null }
    val back = {
        if (!state.busy) {
            when {
                state.pendingRetry -> done()
                state.review != null -> action(RecurrenceIntent.CancelReview)
                editor != null -> discard = true
                else -> done()
            }
        }
    }
    BackHandler { back() }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = {
        discard = false; editor = null; action(RecurrenceIntent.CancelReview)
    })
    Scaffold(modifier = Modifier.fillMaxSize().testTag("recurrence-screen"),
        containerColor = MaterialTheme.colorScheme.background, contentWindowInsets = WindowInsets.safeDrawing,
        topBar = { TopAppBar(title = { Text("Recurring payments") }, navigationIcon = {
            IconButton(onClick = back, enabled = !state.busy) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
        }) }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding()
            .verticalScroll(rememberScrollState()).padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text("Planned payments do not change balances or realized expense reports.")
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
                    state.review?.let { RecurrenceReviewDetails(it, state) }
                    CuenoraAction(if (state.review is RecurrenceReview.Confirm) "Confirm actual payment" else "Apply planning change",
                        { action(RecurrenceIntent.Submit) }, Modifier.testTag("recurrence-submit"), busy = state.busy)
                    TextButton(onClick = { action(RecurrenceIntent.CancelReview) }, enabled = !state.busy) { Text("Keep editing") }
                }
                state.requiresRefresh -> CuenoraAction("Refresh current state", { action(RecurrenceIntent.Refresh) }, busy = state.busy)
                editor != null -> key(editor) {
                    when (val current = editor) {
                        is RecurrenceEditor.Rule -> RuleFields(current.original, state, ruleDraft, { ruleDraft = it }, { terms ->
                            action(current.original?.let { RecurrenceIntent.Edit(it.id, terms) } ?: RecurrenceIntent.Create(terms))
                        })
                        is RecurrenceEditor.Payment -> PaymentFields(current.occurrence, state, paymentDraft, { paymentDraft = it }) { actual -> action(RecurrenceIntent.Confirm(current.occurrence.id, actual)) }
                        is RecurrenceEditor.Postpone -> PostponeFields(current.occurrence, state.busy, postponedDate, { postponedDate = it }) { date -> action(RecurrenceIntent.Postpone(current.occurrence.id, date)) }
                        null -> Unit
                    }
                    TextButton(onClick = { if (currentPayment(editor)) editor = null else discard = true }, enabled = !state.busy) {
                        Text(if (currentPayment(editor)) "Not paid yet" else "Cancel changes")
                    }
                }
                else -> {
                    TextButton(onClick = { action(RecurrenceIntent.Refresh) }, enabled = !state.busy) { Text("Refresh") }
                    if (state.writeSucceeded) CuenoraNotice("Saved", "The server confirmed the operation. Lists show current server data after refresh.", NoticeTone.Success)
                    if (state.profile?.emailVerified == true && !state.busy && state.notice == null) {
                        CuenoraAction("Create recurring expense", { editor = RecurrenceEditor.Rule(null) })
                    }
                    Text("Upcoming and past occurrences", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
                    RecurrenceChoice("Occurrence state", state.occurrenceFilter?.name?.lowercase() ?: "all",
                        listOf("planned", "confirmed", "skipped", "all"), !state.busy) { index ->
                        action(RecurrenceIntent.OccurrenceFilter(OccurrenceState.entries.getOrNull(index)))
                    }
                    if (state.occurrences.isEmpty() && !state.busy && state.notice == null)
                        Text("No occurrences in this view. Generation is a separate server job; creating a rule does not immediately post or generate a payment.")
                    state.occurrences.forEach { occurrence ->
                        HorizontalDivider()
                        Text(occurrence.description ?: "Recurring expense", style = MaterialTheme.typography.titleMedium)
                        Text("Planned: ${accountAmount(occurrence.amount)} ${occurrence.amount.currency.name}")
                        Text("Scheduled: ${recurrenceDate(occurrence.scheduledOn)} · Due: ${recurrenceDate(occurrence.dueOn)}")
                        Text("State: ${occurrence.state.name.lowercase()} · Account: ${accountName(state, occurrence.accountId)}")
                        if (occurrence.state == OccurrenceState.PLANNED && !state.busy && state.notice == null) {
                            TextButton(onClick = { editor = RecurrenceEditor.Payment(occurrence) }) { Text("Review actual payment") }
                            TextButton(onClick = { action(RecurrenceIntent.Skip(occurrence.id)) }) { Text("Skip occurrence") }
                            TextButton(onClick = { editor = RecurrenceEditor.Postpone(occurrence) }) { Text("Postpone occurrence") }
                        }
                    }
                    if (state.hasMoreOccurrences) TextButton(onClick = { action(RecurrenceIntent.MoreOccurrences) }, enabled = !state.busy) { Text("Load more occurrences") }
                    Text("Recurring rules", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
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
            }
        }
    }
}

private fun currentPayment(editor: RecurrenceEditor?) = editor is RecurrenceEditor.Payment

@Composable
private fun RuleFields(original: RecurrenceRule?, state: RecurrenceScreenState, draft: RecurrenceRuleDraft,
    update: (RecurrenceRuleDraft) -> Unit, review: (RecurrenceRuleDraft) -> Unit) {
    Text(if (original == null) "Create recurring expense" else "Edit recurring rule", style = MaterialTheme.typography.headlineSmall)
    CuenoraField("Planned amount", draft.amount, { update(draft.copy(amount = it)) }, Modifier.testTag("recurrence-rule-amount"))
    RecurrenceChoice("Currency", draft.currency.name, Currency.entries.map { it.name }, !state.busy) { update(draft.copy(currency = Currency.entries[it], accountId = null)) }
    RecurrenceChoice("Cadence", draft.cadence, listOf("weekly", "monthly", "quarterly", "annual"), !state.busy) {
        update(draft.copy(cadence = listOf("weekly", "monthly", "quarterly", "annual")[it]))
    }
    CuenoraField("Start date (YYYY-MM-DD)", draft.startsOn, { update(draft.copy(startsOn = it)) }, Modifier.testTag("recurrence-start"))
    CuenoraField("End date (optional, inclusive)", draft.endsOn.orEmpty(), { update(draft.copy(endsOn = it.ifEmpty { null })) }, Modifier.testTag("recurrence-end"))
    CuenoraField("Explicit IANA rule time zone", draft.timeZone, { update(draft.copy(timeZone = it)) }, Modifier.testTag("recurrence-zone"))
    Text("The rule keeps its own time zone. Month-end adjustments preserve the original calendar anchor.")
    RecurrenceReferences(state, draft.currency, draft.accountId, draft.categoryId,
        { update(draft.copy(accountId = it)) }, { update(draft.copy(categoryId = it)) })
    CuenoraField("Description (optional)", draft.description.orEmpty(), { update(draft.copy(description = it.ifEmpty { null })) }, Modifier.testTag("recurrence-description"))
    Text("Edits apply only to new occurrences. Existing planned, skipped and confirmed items are preserved.")
    CuenoraAction("Review rule", { review(draft) }, busy = state.busy)
}

@Composable
private fun PaymentFields(occurrence: RecurrenceOccurrence, state: RecurrenceScreenState, draft: ActualPaymentDraft,
    update: (ActualPaymentDraft) -> Unit, review: (ActualPaymentDraft) -> Unit) {
    // Suggestions are never submissions: amount and actual date must be entered explicitly.
    Text("Record an actual payment", style = MaterialTheme.typography.headlineSmall)
    Text("Planned suggestion: ${accountAmount(occurrence.amount)} ${occurrence.amount.currency.name}, due ${recurrenceDate(occurrence.dueOn)}. Enter what actually happened.")
    CuenoraField("Actual amount", draft.amount, { update(draft.copy(amount = it)) }, Modifier.testTag("recurrence-actual-amount"))
    RecurrenceChoice("Actual currency", draft.currency.name, Currency.entries.map { it.name }, !state.busy) { update(draft.copy(currency = Currency.entries[it], accountId = null)) }
    CuenoraField("Actual payment date (YYYY-MM-DD)", draft.occurredOn, { update(draft.copy(occurredOn = it)) }, Modifier.testTag("recurrence-actual-date"))
    Text("Profile time zone: ${state.profile?.timeZone ?: "Not confirmed; return to Home"}")
    RecurrenceReferences(state, draft.currency, draft.accountId, draft.categoryId,
        { update(draft.copy(accountId = it)) }, { update(draft.copy(categoryId = it)) })
    CuenoraField("Actual description (optional)", draft.description.orEmpty(), { update(draft.copy(description = it.ifEmpty { null })) }, Modifier.testTag("recurrence-actual-description"))
    Text("This records a payment already made. It does not instruct a bank to pay.")
    CuenoraAction("Review actual terms", { review(draft) }, busy = state.busy)
}

@Composable
private fun PostponeFields(occurrence: RecurrenceOccurrence, busy: Boolean, date: String, update: (String) -> Unit, review: (String) -> Unit) {
    Text("Postpone one occurrence", style = MaterialTheme.typography.headlineSmall)
    Text("Current due date: ${recurrenceDate(occurrence.dueOn)}. Choose a strictly later date; the scheduled identity and rule calendar stay unchanged.")
    CuenoraField("New due date (YYYY-MM-DD)", date, update, Modifier.testTag("recurrence-postpone-date"))
    CuenoraAction("Review postponement", { review(date) }, busy = busy)
}

@Composable
private fun RecurrenceReferences(state: RecurrenceScreenState, currency: Currency, account: AccountId?, category: CategoryId?,
    selectAccount: (AccountId?) -> Unit, selectCategory: (CategoryId?) -> Unit) {
    val eligible = state.accounts.map { it.account }.filter { it.status == AccountStatus.ACTIVE && it.currency == currency }
    RecurrenceChoice("Account", accountName(state, account), listOf("No account assigned") + eligible.map { it.name.value }, !state.busy) {
        selectAccount(if (it == 0) null else eligible[it - 1].id)
    }
    val categories = state.categories.filter { it.kind == TransactionKind.EXPENSE }
    RecurrenceChoice("Expense category", categories.firstOrNull { it.id == category }?.name ?: "No category",
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
    Text("Review before applying", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
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
            Text(if (review.actual.accountId == null) "Included in expense reports; no account balance changes." else "Creates one expense and decreases the selected account's recorded balance.")
            Text("Only this occurrence becomes confirmed. This does not execute a bank payment.")
        }
        is RecurrenceReview.Skip -> Text("Skip only this occurrence. No expense or balance movement is created. This cannot be reopened.")
        is RecurrenceReview.Postpone -> Text("Move only this occurrence's due date to ${review.dueOn}. No expense or balance movement is created; its original scheduled identity and rule calendar stay unchanged.")
    }
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
private fun accountName(state: RecurrenceScreenState, id: AccountId?) = if (id == null) "No account assigned"
    else state.accounts.firstOrNull { it.account.id == id }?.account?.name?.value ?: "Assigned account (name unavailable)"
