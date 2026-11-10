package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.foundation.text.input.TextObfuscationMode
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.LifecycleOwner
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.validation.AccountForm
import java.util.TimeZone

@Composable
fun OnboardingRoot(flow: OnboardingSession, accountsContent: (@Composable () -> Unit)? = null, transactionsContent: (@Composable () -> Unit)? = null, recordActivity: (() -> Unit)? = null, recurrenceActivity: (() -> Unit)? = null,
    homeContent: (@Composable () -> Unit)? = null, insightsContent: (@Composable () -> Unit)? = null,
    addAccount: (() -> Unit)? = null, addAccountEnabled: Boolean = true, submit: (suspend () -> Unit) -> Unit) {
    val state by flow.state.collectAsState()
    LaunchedEffect(flow) { submit { flow.load() } }
    CuenoraApp(accountsContent = accountsContent, transactionsContent = transactionsContent, insightsContent = insightsContent) {
        val navigate = LocalDestinationNavigation.current
        Column(Modifier.fillMaxSize()) {
            // Compact, wrapping actions leave the report its own bounded scroll area.
            FlowRow(Modifier.padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (recordActivity != null) TextButton(onClick = recordActivity,
                    modifier = Modifier.testTag("record-from-home")) { Text("Record activity") }
                if (recurrenceActivity != null) TextButton(onClick = recurrenceActivity,
                    modifier = Modifier.testTag("recurrence-from-home")) { Text("Recurring payments") }
                if (homeContent != null && addAccount != null) TextButton(onClick = addAccount, enabled = addAccountEnabled) {
                    Text("Add account")
                }
                if (homeContent != null) TextButton(onClick = { navigate(Destination.Accounts) }) { Text("Accounts and savings") }
            }
            Box(Modifier.weight(1f)) {
                if (homeContent != null && state.stage in setOf(OnboardingStage.Existing, OnboardingStage.Complete, OnboardingStage.Empty))
                    homeContent()
                else OnboardingPanel(state, flow, submit, accountsContent != null)
            }
        }
    }
}

@Composable
internal fun OnboardingPanel(state: OnboardingState, flow: OnboardingSession, submit: (suspend () -> Unit) -> Unit,
    accountsConnected: Boolean, additional: Boolean = false, requestCancel: (() -> Unit)? = null) {
    var discard by remember { mutableStateOf(false) }
    val canCancel = state.stage in setOf(OnboardingStage.Form, OnboardingStage.Review) && !state.busy
    BackHandler(enabled = canCancel && requestCancel == null) { discard = true }
    if (discard) CuenoraDiscardDialog(onKeepEditing = { discard = false }, onDiscard = {
        discard = false; flow.cancel()
    })
    Column(Modifier.fillMaxSize().imePadding().verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)) {
        when (state.stage) {
            OnboardingStage.Loading -> CuenoraLoading("Checking your profile and accounts…")
            OnboardingStage.Verify -> VerificationPanel(state, flow, submit)
            OnboardingStage.Empty -> {
                Title("Your first account")
                Text("Keep a clear starting point for your money. Add a normal or savings account when you are ready.")
                CuenoraAction("Add first account", flow::begin, Modifier.testTag("onboarding-begin"))
                Text("An account is optional for income and expense records. Use Record activity to record without one.")
            }
            OnboardingStage.Existing, OnboardingStage.Complete -> {
                Title(if (state.stage == OnboardingStage.Complete) "Your account is ready" else "Welcome back")
                Text(if (state.stage == OnboardingStage.Complete)
                    "The account was saved. An opening balance is a starting movement, not income."
                    else "You already have an account. First-account setup is not required.")
                Text(if (accountsConnected) "Open Accounts to view account details and posted balances."
                    else "Account lists and details will be connected in P11.04.")
            }
            OnboardingStage.Form -> AccountPanel(state, flow, submit, additional)
            OnboardingStage.Review -> {
                Title("Review your account")
                state.form?.let { form ->
                    Text("Name: ${form.name.trim()}")
                    Text("Type: ${if (form.type == AccountType.NORMAL) "Normal" else "Savings"}")
                    Text("Currency: ${form.currency.name}")
                    if (form.openingBalanceAmount != null) {
                        CuenoraAmount("Opening balance", form.currency.name, form.openingBalanceAmount.orEmpty())
                        Text("Business date: ${form.openingBalanceOccurredOn}")
                        Text("Profile time zone: ${state.profile?.timeZone}")
                        Text("This is not income. No movement is posted for a zero opening balance.")
                    } else Text("No opening balance movement.")
                }
                CuenoraAction("Create account", { submit { flow.save() } }, Modifier.testTag("onboarding-save"), busy = state.busy)
                TextButton(onClick = flow::edit, enabled = !state.busy) { Text("Keep editing") }
            }
            OnboardingStage.Uncertain -> {
                Title("Check the original account creation")
                Text("Completion has not been confirmed. Retry the original request safely; do not create another account. The original details and operation key are preserved securely.")
                CuenoraAction("Retry original request", { submit { flow.save() } }, Modifier.testTag("onboarding-retry-write"), busy = state.busy)
            }
            OnboardingStage.Failed -> {
                Title("Setup could not be loaded")
                Text("No empty account list or successful save is assumed. Retry to check the current server and secure-storage state.")
                CuenoraAction("Try again", { submit { flow.load() } }, Modifier.testTag("onboarding-reload"), busy = state.busy)
            }
        }
        state.notice?.let { notice ->
            CuenoraNotice(if (notice == OnboardingNotice.MailAccepted) "Request accepted" else "Setup status", noticeCopy(notice),
                if (notice == OnboardingNotice.Saved) NoticeTone.Success else if (notice == OnboardingNotice.MailAccepted)
                    NoticeTone.Information else NoticeTone.Attention)
        }
        if (canCancel) TextButton(onClick = { if (requestCancel != null) requestCancel() else discard = true },
            modifier = Modifier.testTag("onboarding-cancel")) { Text(if (additional) "Cancel account" else "Cancel setup") }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun VerificationPanel(state: OnboardingState, flow: OnboardingSession, submit: (suspend () -> Unit) -> Unit) {
    val token = remember { TextFieldState() }
    fun clearToken() { token.edit { replace(0, length, "") } }
    val owner = LocalContext.current as? LifecycleOwner
    DisposableEffect(owner) {
        val observer = LifecycleEventObserver { _, event -> if (event == Lifecycle.Event.ON_STOP) clearToken() }
        owner?.lifecycle?.addObserver(observer)
        onDispose { clearToken(); owner?.lifecycle?.removeObserver(observer) }
    }
    Title("Verify your email")
    Text("Before creating financial accounts, verify the email for the account you signed in to.")
    CuenoraAction("Request verification email", { submit { flow.requestVerification() } },
        Modifier.testTag("verification-request"), busy = state.busy)
    Text("Open the email link in your browser and sign in to the same account there, then check again below. If your email includes a verification token, you can confirm it here instead.")
    OutlinedSecureTextField(state = token, enabled = !state.busy,
        label = { Text("Verification token") }, supportingText = { Text("Paste the ev1_ token from your email. It is not saved.") },
        textObfuscationMode = TextObfuscationMode.Hidden, modifier = Modifier.fillMaxWidth().testTag("verification-token"))
    CuenoraAction("Confirm token", {
        val submitted = token.text.toString(); clearToken(); submit { flow.confirmVerification(submitted) }
    }, Modifier.testTag("verification-confirm"), unavailableReason = if (token.text.isEmpty()) "Enter a token to confirm it." else null, busy = state.busy)
    TextButton(onClick = { submit { flow.load() } }, enabled = !state.busy, modifier = Modifier.testTag("verification-check")) {
        Text("I verified my email — check again")
    }
}

@Composable
private fun AccountPanel(state: OnboardingState, flow: OnboardingSession, submit: (suspend () -> Unit) -> Unit, additional: Boolean) {
    val form = state.form ?: return
    var zone by remember(state.profile?.timeZone) { mutableStateOf(state.profile?.timeZone ?: TimeZone.getDefault().id) }
    Title(if (additional) "Account details" else "Add your first account")
    CuenoraField("Account name", form.name, { flow.updateForm(form.copy(name = it)) }, Modifier.testTag("account-name"),
        error = if (state.issues.any { it.field == "name" }) "Enter a valid account name (1–255 characters)." else null)
    Text("Account type")
    Row {
        AccountType.entries.forEach { type ->
            FilterChip(selected = form.type == type, onClick = { if (!state.busy) flow.updateForm(form.copy(type = type)) },
                label = { Text(if (type == AccountType.NORMAL) "Normal" else "Savings") }, enabled = !state.busy)
            Spacer(Modifier.width(8.dp))
        }
    }
    Text("Currency")
    Row {
        Currency.entries.forEach { currency ->
            FilterChip(selected = form.currency == currency, enabled = !state.busy,
                onClick = { flow.updateForm(form.copy(currency = currency)) }, label = { Text(currency.name) })
            Spacer(Modifier.width(8.dp))
        }
    }
    Row {
        Checkbox(checked = form.openingBalanceAmount != null, enabled = !state.busy, onCheckedChange = { checked ->
            flow.updateForm(form.copy(openingBalanceAmount = if (checked) "0.00" else null,
                openingBalanceOccurredOn = if (checked) "" else null))
        }, modifier = Modifier.testTag("account-opening-enabled"))
        Text("Include an opening balance", Modifier.padding(top = 12.dp))
    }
    if (state.profile?.timeZone == null) {
        Text("Confirm your profile time zone. The device zone is only a suggestion, not a saved preference.")
        CuenoraField("IANA time zone", zone, { zone = it }, Modifier.testTag("account-zone"), supportingText = "For example, Europe/Stockholm")
        CuenoraAction("Confirm time zone", { submit { flow.confirmZone(zone) } }, Modifier.testTag("account-confirm-zone"), busy = state.busy)
    } else Text("Confirmed profile time zone: ${state.profile?.timeZone}. Device changes do not replace it.")
    if (form.openingBalanceAmount != null) {
        CuenoraField("Opening balance", form.openingBalanceAmount.orEmpty(), {
            flow.updateForm(form.copy(openingBalanceAmount = it))
        }, Modifier.testTag("account-opening-amount"), supportingText = "Use a decimal point and at most two decimal places. Negative balances are allowed.")
        CuenoraField("Opening date", form.openingBalanceOccurredOn.orEmpty(), {
            flow.updateForm(form.copy(openingBalanceOccurredOn = it))
        }, Modifier.testTag("account-opening-date"), supportingText = "YYYY-MM-DD, not later than today in your profile zone.")
    }
    Text("Savings funding after opening uses transfers or credited interest, not ordinary income.")
    CuenoraAction("Review account", { flow.review(form) }, Modifier.testTag("account-review"),
        unavailableReason = if (state.busy) "Wait for the current request to finish." else null)
}

@Composable
private fun Title(text: String) {
    Text(text, style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
}

private fun noticeCopy(notice: OnboardingNotice): String = when (notice) {
    OnboardingNotice.MailAccepted -> "The request was accepted. Delivery and verification are not yet confirmed. You can request another email if needed."
    OnboardingNotice.InvalidChallenge -> "The verification token was not accepted. Use an unexpired token for this signed-in account or request another email."
    OnboardingNotice.InvalidForm -> "Check the account name, exact opening amount and business date. No successful financial write is assumed."
    OnboardingNotice.ZoneRequired -> "Explicitly confirm your profile time zone before reviewing a dated opening balance."
    OnboardingNotice.NetworkUnavailable -> "Connection unavailable. Retry when connected; an uncertain account request keeps its original operation key."
    OnboardingNotice.AccessDenied -> "Access is unavailable. Sign in to the same account and verify its email before retrying."
    OnboardingNotice.RateLimited -> "Too many requests. Wait before trying again."
    OnboardingNotice.ServiceUnavailable -> "The service could not confirm this request. Try again later."
    OnboardingNotice.StorageUnavailable -> "Secure recovery storage is unavailable. No new account request will be sent until it can be checked."
    OnboardingNotice.WriteUncertain -> "The account creation outcome is unknown. Retry only the original request."
    OnboardingNotice.Saved -> "Account creation was confirmed by the server."
}
