package org.cuenora.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.foundation.text.input.TextObfuscationMode
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import org.cuenora.shared.auth.*

/** The session gate lives outside the financial navigation graph; Back cannot bypass it. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AuthenticationRoot(session: AuthSession, submit: (suspend () -> Unit) -> Unit) {
    val state by session.state.collectAsState()
    LaunchedEffect(session) { submit { session.restore() } }
    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        when (state.phase) {
            AuthPhase.Restoring -> Box(Modifier.safeDrawingPadding(), contentAlignment = Alignment.Center) {
                Text("Checking your session…")
            }
            AuthPhase.SignedIn, AuthPhase.Renewing -> Column {
                TextButton(
                    enabled = state.phase == AuthPhase.SignedIn,
                    onClick = { submit { session.logout() } },
                    modifier = Modifier.statusBarsPadding().align(Alignment.End).testTag("auth-sign-out").cuenoraFocus(),
                ) { Text(if (state.phase == AuthPhase.Renewing) "Renewing session…" else "Sign out") }
                Box(Modifier.weight(1f)) { CuenoraApp() }
            }
            else -> AuthForm(state, session, submit)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun AuthForm(state: AuthState, session: AuthSession, submit: (suspend () -> Unit) -> Unit) {
    var registering by remember { mutableStateOf(false) }
    var email by remember { mutableStateOf("") }
    // Do not use rememberSaveable/rememberTextFieldState: passwords must not enter saved state.
    val password = remember { TextFieldState() }
    val focus = LocalFocusManager.current
    val lifecycleOwner = LocalContext.current as? LifecycleOwner
    fun clearPassword() { password.edit { replace(0, length, "") } }
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP) clearPassword()
        }
        lifecycleOwner?.lifecycle?.addObserver(observer)
        onDispose {
            lifecycleOwner?.lifecycle?.removeObserver(observer)
            clearPassword()
        }
    }
    val busy = state.phase == AuthPhase.Working
    val blocked = state.phase == AuthPhase.StorageBlocked
    Box(Modifier.fillMaxSize().safeDrawingPadding().imePadding()) {
        Column(
            modifier = Modifier.align(Alignment.TopCenter).widthIn(max = 480.dp).fillMaxWidth()
                .verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text("Cuenora", style = MaterialTheme.typography.displayMedium)
            Text(if (registering) "Create your access" else "Welcome back",
                style = MaterialTheme.typography.headlineLarge, modifier = Modifier.semantics { heading() })
            Text("Your finances, clearly organized.")
            state.notice?.let { notice ->
                val (title, message) = noticeCopy(notice)
                CuenoraNotice(title, message,
                    if (notice == AuthNotice.RegistrationAccepted) NoticeTone.Information else NoticeTone.Attention)
            }
            if (blocked) {
                CuenoraAction("Clear local session", { submit { session.logout() } })
                Text("Access is blocked until secure storage can be cleared. No plaintext fallback is used.")
            } else {
                val emailError = state.issues.firstOrNull { it.field == "email" }
                val passwordError = state.issues.firstOrNull { it.field == "password" }
                OutlinedTextField(
                    value = email, onValueChange = { email = it }, label = { Text("Email") },
                    enabled = !busy, singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email, autoCorrectEnabled = false),
                    isError = emailError != null,
                    supportingText = { if (emailError != null) Text("Enter a valid email address.") },
                    modifier = Modifier.fillMaxWidth().testTag("auth-email").cuenoraFocus(),
                    shape = RoundedCornerShape(10.dp),
                )
                OutlinedSecureTextField(
                    state = password, label = { Text("Password") }, enabled = !busy,
                    textObfuscationMode = TextObfuscationMode.Hidden,
                    isError = passwordError != null,
                    supportingText = {
                        Text(if (passwordError != null) "Check the password length and characters."
                            else if (registering) "Use 15–128 characters. Common passwords are not accepted."
                            else "Use your existing password.")
                    },
                    modifier = Modifier.fillMaxWidth().testTag("auth-password").cuenoraFocus(),
                    shape = RoundedCornerShape(10.dp),
                )
                Button(
                    enabled = !busy,
                    onClick = {
                        val secret = password.text.toString()
                        clearPassword()
                        focus.clearFocus()
                        submit {
                            if (registering) session.register(email, secret) else session.login(email, secret)
                        }
                    },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).testTag("auth-submit").cuenoraFocus(),
                ) { Text(if (busy) "Please wait…" else if (registering) "Register" else "Sign in") }
                TextButton(
                    enabled = !busy,
                    onClick = { registering = !registering; clearPassword(); focus.clearFocus() },
                    modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).testTag("auth-switch").cuenoraFocus(),
                ) { Text(if (registering) "Already registered? Sign in" else "New here? Register") }
                if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                Text("Email verification and financial setup are separate steps. These screens do not create an account balance.")
            }
        }
    }
}

private fun noticeCopy(notice: AuthNotice): Pair<String, String> = when (notice) {
    AuthNotice.RegistrationAccepted -> "Registration accepted" to
        "You can try signing in with your credentials. This response does not confirm a new account or email delivery."
    AuthNotice.InvalidCredentials -> "Could not sign in" to "Email or password was not accepted."
    AuthNotice.RateLimited -> "Please wait" to "Too many attempts. Wait before trying again."
    AuthNotice.NetworkUnavailable -> "Connection unavailable" to "Check your connection, then try again."
    AuthNotice.ServiceUnavailable -> "Access unavailable" to "The request could not be completed. Try again later."
    AuthNotice.SessionExpired -> "Sign in again" to "Your saved session is no longer accepted."
    AuthNotice.SessionUncertain -> "Sign in again" to
        "Session renewal could not be confirmed. For safety, the saved session was removed."
    AuthNotice.StorageUnavailable -> "Secure storage unavailable" to "Your local session could not be safely stored or cleared."
    AuthNotice.RemoteLogoutUnconfirmed -> "Signed out on this device" to
        "Local credentials were removed, but server-side sign-out could not be confirmed."
}
