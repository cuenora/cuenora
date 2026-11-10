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
import org.cuenora.shared.repository.*
import org.cuenora.shared.savings.*

sealed interface SavingsIntent {
    data class Review(val action: SavingsAction) : SavingsIntent
    data class Forecast(val start: String, val end: String) : SavingsIntent
    data object Confirm : SavingsIntent
    data object Edit : SavingsIntent
    data object Refresh : SavingsIntent
    data object MoreTerms : SavingsIntent
    data object MoreRates : SavingsIntent
}

@Composable
fun SavingsScreen(flow: SavingsSession, submit: (suspend () -> Unit) -> Unit, changed: () -> Unit, done: () -> Unit) {
    val state by flow.state.collectAsState()
    LaunchedEffect(state.stage) { if (state.stage == SavingsStage.Saved) changed() }
    SavingsPanel(state, { intent ->
        when (intent) {
            is SavingsIntent.Review -> flow.review(intent.action)
            is SavingsIntent.Forecast -> submit { flow.forecast(intent.start, intent.end) }
            SavingsIntent.Confirm -> submit { flow.confirm() }
            SavingsIntent.Edit -> flow.edit()
            SavingsIntent.Refresh -> state.account?.let { account -> submit { flow.open(account) } }
            SavingsIntent.MoreTerms -> submit { flow.moreTerms() }
            SavingsIntent.MoreRates -> submit { flow.moreRates() }
        }
    }, done)
}

/** Memory-only drafts; estimates can never be converted into a receipt by an automatic action. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SavingsPanel(state: SavingsState, act: (SavingsIntent) -> Unit, done: () -> Unit) {
    var tab by remember { mutableStateOf("Overview") }
    var discard by remember { mutableStateOf(false) }
    var dirty by remember { mutableStateOf(false) }
    val draft = remember { SavingsDraft() }
    LaunchedEffect(state.stage) { if (state.stage == SavingsStage.Saved) dirty = false }
    val back = {
        if (!state.busy) {
            if (dirty || state.stage == SavingsStage.Review) discard = true else done()
        }
    }
    BackHandler { back() }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = { discard = false; done() })
    Scaffold(Modifier.fillMaxSize().testTag("savings-screen"), contentWindowInsets = WindowInsets.safeDrawing,
        topBar = { TopAppBar(title = { Text("Savings") }, navigationIcon = {
            IconButton(onClick = back, enabled = !state.busy) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
        }, colors = TopAppBarDefaults.topAppBarColors(containerColor = MaterialTheme.colorScheme.background)) }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding()
            .verticalScroll(rememberScrollState()).padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(state.account?.name?.value ?: "Savings", style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() })
            when (state.stage) {
                SavingsStage.Loading -> CuenoraLoading("Reading savings and checking secure recovery…")
                SavingsStage.Browsing -> {
                    // A vertical selector also works with large fonts and narrow windows.
                    listOf("Overview", "Terms and rates", "Forecast", "Actual receipt").forEach { label ->
                        FilterChip(selected = tab == label, onClick = { tab = label }, label = { Text(label) })
                    }
                    TextButton(onClick = { dirty = false; act(SavingsIntent.Refresh) }, enabled = !state.busy) { Text("Refresh savings") }
                    when (tab) {
                        "Terms and rates" -> TermsAndRates(state, act, draft) { dirty = true }
                        "Forecast" -> ForecastFields(state, act, draft) { dirty = true }
                        "Actual receipt" -> ReceiptFields(state, act, draft) { dirty = true }
                        else -> {
                            Text("Savings terms and rates are optional, effective-dated versions. Adding a version never posts money.")
                            Text("Forecasts are estimates. Record actual interest only after the bank has credited the receiving account.")
                            Text("Savings funding uses transfers, not ordinary income.")
                        }
                    }
                }
                SavingsStage.Review, SavingsStage.Uncertain -> {
                    Text(if (state.stage == SavingsStage.Uncertain) "Resolve original request" else "Review savings request",
                        style = MaterialTheme.typography.titleLarge)
                    state.action?.let { SavingsReview(it, state) }
                    CuenoraAction(if (state.stage == SavingsStage.Uncertain) "Retry original request"
                        else if (state.action is SavingsAction.Receipt) "Confirm actual receipt" else "Apply configuration version",
                        { act(SavingsIntent.Confirm) }, busy = state.busy, modifier = Modifier.testTag("savings-confirm"))
                    if (state.stage == SavingsStage.Review) TextButton(onClick = { act(SavingsIntent.Edit) }, enabled = !state.busy) { Text("Keep editing") }
                    else Text("The original account, date and details are retained securely. Receipts retain their original retry key. Nothing is replayed automatically; leaving does not discard the request.")
                }
                SavingsStage.Saved -> {
                    Text("Request confirmed. Do not create a second receipt for the same bank credit.")
                    CuenoraAction("Reload savings", { dirty = false; act(SavingsIntent.Refresh) })
                    CuenoraAction("Done", done)
                }
                SavingsStage.Failed -> CuenoraAction("Try again", { act(SavingsIntent.Refresh) })
                SavingsStage.Blocked -> Text("Return to Home to verify your email and confirm your profile time zone.")
                SavingsStage.Closed -> Unit
            }
            state.notice?.let { CuenoraNotice("Savings status", it, NoticeTone.Attention) }
            if (state.busy && state.stage == SavingsStage.Browsing) CuenoraLoading("Reading savings…")
        }
    }
}

@Composable
private fun TermsAndRates(state: SavingsState, act: (SavingsIntent) -> Unit, draft: SavingsDraft, dirty: () -> Unit) {
    var error by remember { mutableStateOf<String?>(null) }
    Text("Immutable terms history", style = MaterialTheme.typography.titleLarge)
    Text("Versions cannot be edited or deleted, including future versions. A different value at an existing date conflicts. Confirm each date carefully.")
    if (state.terms.isEmpty()) Text("No configured terms.")
    state.terms.forEach { version ->
        Text("${savingsDate(version.terms.effectiveOn)} · ${version.terms.payoutFrequency} · projected capitalization: ${if (version.terms.capitalize) "reinvest" else "exclude"}")
    }
    if (state.termsCursor != null) TextButton(onClick = { act(SavingsIntent.MoreTerms) }, enabled = !state.busy) { Text("Load more terms") }
    Text("Nominal annual rate history", style = MaterialTheme.typography.titleLarge)
    if (state.rates.isEmpty()) Text("No configured rates.")
    state.rates.forEach { Text("${savingsDate(it.rate.effectiveOn)} · ${it.rate.nominalAnnualPercent}% nominal annual") }
    if (state.ratesCursor != null) TextButton(onClick = { act(SavingsIntent.MoreRates) }, enabled = !state.busy) { Text("Load more rates") }
    Text("Add terms version", style = MaterialTheme.typography.titleLarge)
    CuenoraField("Effective date (YYYY-MM-DD)", draft.termsDate, { draft.termsDate = it; dirty() }, Modifier.testTag("savings-effective"))
    Text("Payout frequency")
    listOf("daily", "monthly", "quarterly", "annual").forEach { value ->
        FilterChip(selected = draft.frequency == value, onClick = { draft.frequency = value; dirty() }, label = { Text(value.replaceFirstChar { it.uppercase() }) })
    }
    Text("Projected capitalization (choose explicitly)")
    FilterChip(selected = draft.capitalize == true, onClick = { draft.capitalize = true; dirty() }, label = { Text("Reinvest projected payouts") })
    FilterChip(selected = draft.capitalize == false, onClick = { draft.capitalize = false; dirty() }, label = { Text("Exclude projected payouts") })
    Text("Nominal annual rate; actual calendar-year 365/366 days; daily closing balance; natural payout periods; period-end half-even rounding. These are Cuenora's estimate policies, not inferred bank terms.")
    CuenoraAction("Review terms", {
        val parsed = savingsDateOrNull(draft.termsDate)
        if (parsed == null || draft.frequency == null || draft.capitalize == null) error = "Enter a valid date and choose payout frequency and capitalization."
        else { error = null; act(SavingsIntent.Review(SavingsAction.Terms(state.account!!.id, SavingsTermsCommand(parsed, draft.frequency!!, draft.capitalize!!)))) }
    }, busy = state.busy)
    Text("Add rate version", style = MaterialTheme.typography.titleLarge)
    Text("The same explicit effective date above applies to this independent rate version.")
    CuenoraField("Nominal annual percentage", draft.rate, { draft.rate = it; dirty() }, Modifier.testTag("savings-rate"))
    CuenoraAction("Review rate", {
        val command = runCatching { SavingsRateCommand(savingsDateOrNull(draft.termsDate) ?: error("date"), draft.rate) }.getOrNull()
        if (command == null) error = "Enter a valid date and a nonnegative nominal annual percentage with at most six decimal places."
        else { error = null; act(SavingsIntent.Review(SavingsAction.Rate(state.account!!.id, command))) }
    }, busy = state.busy)
    error?.let { Text(it) }
}

@Composable
private fun ForecastFields(state: SavingsState, act: (SavingsIntent) -> Unit, draft: SavingsDraft, dirty: () -> Unit) {
    Text("Choose an inclusive 1–366 day forecast range, starting today or later in your confirmed profile zone.")
    CuenoraField("Start date (YYYY-MM-DD)", draft.start, { draft.start = it; dirty() }, Modifier.testTag("savings-start"))
    CuenoraField("End date (YYYY-MM-DD)", draft.end, { draft.end = it; dirty() }, Modifier.testTag("savings-end"))
    CuenoraAction("Calculate estimate", { act(SavingsIntent.Forecast(draft.start, draft.end)) }, busy = state.busy)
    Text("Estimate only — no balance or realized income changes.")
    Text("The current posted balance is held flat. Future deposits, withdrawals and actual interest credits are not predicted. Prior accrual is excluded; this is not a guaranteed next bank payout.")
    state.estimate?.let { estimate ->
        Text("Requested range: ${savingsDate(estimate.start)} to ${savingsDate(estimate.end)}")
        CuenoraAmount("Posted snapshot", estimate.currency.name, accountAmount(estimate.postedBalance))
        Text("Snapshot date: ${savingsDate(estimate.balanceAsOf)} · profile zone: ${state.zone ?: "Unavailable"}")
        Text("${estimate.terms.payoutFrequency} payouts · projected capitalization: ${if (estimate.terms.capitalize) "reinvest" else "exclude"}")
        if (estimate.partialFirstPeriod) Text("Partial first period: interest before the start date is not included.")
        Text("Nominal annual percentage, daily closing balance, actual calendar-year days and period-end half-even rounding.")
        estimate.rates.forEach { segment ->
            Text("${savingsDate(segment.start)} to ${savingsDate(segment.end)}: ${segment.nominalAnnualPercent}% nominal annual / ${segment.daysInYear} days (effective ${savingsDate(segment.effectiveOn)})")
        }
        estimate.periods.forEach { period ->
            Text("Period: ${savingsDate(period.start)} to ${savingsDate(period.end)}")
            Text("Exact accrued minor units: ${period.accrual.numerator}/${period.accrual.denominator}")
            Text(period.projectedPayout?.let { "Projected payout: $it ${estimate.currency.name}" } ?: "Open period: no rounded payout yet.")
        }
    }
}

@Composable
private fun ReceiptFields(state: SavingsState, act: (SavingsIntent) -> Unit, draft: SavingsDraft, dirty: () -> Unit) {
    var error by remember { mutableStateOf<String?>(null) }
    Text("Record only interest already credited by the bank. The actual amount may differ from any forecast; terms are not required.")
    Text("Receiving account (choose explicitly)")
    if (state.receivingAccounts.isEmpty()) Text("No receiving accounts available. Refresh savings to retry the account read.")
    state.receivingAccounts.forEach { account ->
        FilterChip(selected = draft.receiver?.id == account.id, onClick = { draft.receiver = account; dirty() },
            label = { Text("${account.name.value} · ${account.currency.name}") })
    }
    CuenoraField("Actual amount credited", draft.amount, { draft.amount = it; dirty() }, Modifier.testTag("savings-actual-amount"))
    CuenoraField("Actual credit date (YYYY-MM-DD)", draft.receiptDate, { draft.receiptDate = it; dirty() }, Modifier.testTag("savings-actual-date"))
    CuenoraField("Description (optional)", draft.description, { draft.description = it; dirty() }, Modifier.testTag("savings-description"))
    Text("Currency: ${draft.receiver?.currency?.name ?: "Choose a receiving account"} · profile zone: ${state.zone ?: "Confirm from Home"}")
    CuenoraAction("Review actual receipt", {
        val account = draft.receiver
        if (account == null) error = "Choose the account that actually received the credit."
        else { error = null; act(SavingsIntent.Review(SavingsAction.Receipt(account.id, draft.amount, account.currency, draft.receiptDate, draft.description.ifEmpty { null }))) }
    }, busy = state.busy)
    error?.let { Text(it) }
}

@Composable
private fun SavingsReview(action: SavingsAction, state: SavingsState) {
    if (action !is SavingsAction.Receipt) Text("Configuration account: " +
        (if (state.account?.id == action.account) state.account!!.name.value
        else "Original savings account (${action.account.highBits}:${action.account.lowBits})"))
    when (action) {
        is SavingsAction.Terms -> {
            Text("Effective date: ${savingsDate(action.command.effectiveOn)}")
            Text("Payout frequency: ${action.command.payoutFrequency}")
            Text("Projected capitalization: ${if (action.command.capitalize) "reinvest" else "exclude"}")
            Text("Nominal annual; actual calendar-year days; daily closing balance; period-end half-even rounding.")
            Text("Adds an immutable terms version. No balance or realized income changes.")
        }
        is SavingsAction.Rate -> {
            Text("Effective date: ${savingsDate(action.command.effectiveOn)}")
            Text("Nominal annual percentage: ${action.command.nominalAnnualPercent}%")
            Text("Adds an immutable rate version. No balance or realized income changes.")
        }
        is SavingsAction.Receipt -> {
            Text("Actual credited interest: ${action.amount} ${action.currency.name}")
            Text("Actual credit date: ${action.occurredOn}")
            Text("Receiving account: ${state.receivingAccounts.firstOrNull { it.id == action.account }?.name?.value ?: "Original account (${action.account.highBits}:${action.account.lowBits})"}")
            Text("Profile zone: ${state.zone ?: "Unavailable"}")
            action.description?.let { Text("Description: $it") }
            Text("Only the receiving account is credited. No second credit is added to savings.")
            Text("Included once in realized interest income. This records a bank credit; it does not move money at a bank.")
        }
    }
}
private fun savingsDate(date: BusinessDate) = "${date.year.toString().padStart(4, '0')}-${date.month.toString().padStart(2, '0')}-${date.day.toString().padStart(2, '0')}"
private fun savingsDateOrNull(text: String): BusinessDate? = runCatching {
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(text))
    BusinessDate(text.take(4).toInt(), text.substring(5, 7).toInt(), text.takeLast(2).toInt())
}.getOrNull()

@Stable
private class SavingsDraft {
    var termsDate by mutableStateOf("")
    var frequency by mutableStateOf<String?>(null)
    var capitalize by mutableStateOf<Boolean?>(null)
    var rate by mutableStateOf("")
    var start by mutableStateOf("")
    var end by mutableStateOf("")
    var receiver by mutableStateOf<Account?>(null)
    var amount by mutableStateOf("")
    var receiptDate by mutableStateOf("")
    var description by mutableStateOf("")
}
