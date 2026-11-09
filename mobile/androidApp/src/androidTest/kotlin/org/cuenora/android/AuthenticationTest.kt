package org.cuenora.android

import androidx.compose.ui.test.*
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.runtime.rememberCoroutineScope
import androidx.test.espresso.Espresso.closeSoftKeyboard
import kotlinx.coroutines.launch
import org.cuenora.android.ui.AuthenticationRoot
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.shared.auth.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class AuthenticationTest {
    @get:Rule val compose = createComposeRule()
    private var calls = 0
    private var lastPassword: String? = null
    private var saved: SessionCredentials? = null
    private var loginFailure = false
    private var networkFailure = false
    private var storageFailure = false
    private val repository = object : AuthRepository {
        override suspend fun register(email: String, password: String): RepositoryResult<Unit> {
            calls++; return RepositoryResult.Success(Unit)
        }
        override suspend fun login(email: String, password: String): RepositoryResult<TokenGrant> {
            calls++; lastPassword = password
            return if (networkFailure) RepositoryResult.Failure(ApiFailure.Network)
            else if (loginFailure) RepositoryResult.Failure(ApiFailure.Http(401, null, null, null))
            else RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-access", "synthetic-refresh"), 900))
        }
        override suspend fun refresh(credentials: SessionCredentials) =
            RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-new-access", "synthetic-new-refresh"), 900))
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }
    private val store = object : SessionStore {
        override suspend fun read(): SessionCredentials? {
            if (storageFailure) throw SessionStoreException(SessionFailure.Corrupt)
            return saved
        }
        override suspend fun replace(credentials: SessionCredentials) { saved = credentials }
        override suspend fun clear() {
            if (storageFailure) throw SessionStoreException(SessionFailure.Unavailable)
            saved = null
        }
    }

    private fun render() {
        val session = AuthSession(repository, store)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { AuthenticationRoot(session) { action -> scope.launch { action() } } }
        }
    }

    private fun submit() {
        // A semantic node can exist below the IME-clipped viewport without receiving a tap.
        closeSoftKeyboard()
        compose.onNodeWithTag("auth-submit").performScrollTo().performClick()
    }

    @Test fun signedOutGateValidatesThenLogsInAndCanSignOut() {
        render()
        submit()
        compose.onNodeWithText("Enter a valid email address.", substring = true).assertExists()
        assertEquals(0, calls)
        compose.onNodeWithTag("auth-email").performTextInput("person@example.test")
        compose.onNodeWithTag("auth-password").performTextInput(" x ")
        submit()
        compose.onNodeWithTag("auth-sign-out").assertIsNotDisplayed()
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("auth-sign-out").assertExists()
        assertEquals(" x ", lastPassword)
        assertNotNull(saved)
        compose.onNodeWithTag("auth-sign-out").performClick()
        compose.onNodeWithTag("auth-submit").assertExists()
        assertEquals("", compose.onNodeWithTag("auth-password").fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
        assertNull(saved)
    }

    @Test fun registrationAcceptanceDoesNotSignInOrPromiseAnEmail() {
        render()
        compose.onNodeWithTag("auth-switch").performClick()
        compose.onNodeWithTag("auth-email").performTextInput("person@example.test")
        compose.onNodeWithTag("auth-password").performTextInput("a synthetic password")
        submit()
        compose.onNodeWithText("Registration accepted", substring = true).assertExists()
        compose.onNodeWithTag("auth-sign-out").assertIsNotDisplayed()
        assertNull(saved)
    }

    @Test fun failedLoginShowsFixedErrorAndRemovesPassword() {
        loginFailure = true
        render()
        compose.onNodeWithTag("auth-email").performTextInput("person@example.test")
        compose.onNodeWithTag("auth-password").performTextInput("synthetic-secret")
        submit()
        compose.onNodeWithText("Email or password was not accepted.", substring = true).assertExists()
        assertEquals("", compose.onNodeWithTag("auth-password").fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
        assertNull(saved)
    }

    @Test fun storedSessionIsRestoredThroughRefreshBeforeShowingTheShell() {
        saved = SessionCredentials("synthetic-old-access", "synthetic-old-refresh")
        render()
        compose.onNodeWithTag("auth-sign-out").assertIsNotDisplayed()
        compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("auth-sign-out").assertExists()
        assertEquals("synthetic-new-refresh", saved?.refreshToken)
    }
    @Test fun networkFailureOffersRetryWithoutOpeningTheFinancialShell() {
        networkFailure = true
        render()
        compose.onNodeWithTag("auth-email").performTextInput("person@example.test")
        compose.onNodeWithTag("auth-password").performTextInput("synthetic-secret")
        submit()
        compose.onNodeWithText("Connection unavailable", substring = true).assertExists()
        compose.onNodeWithTag("auth-submit").assertIsEnabled()
        compose.onNodeWithTag("auth-sign-out").assertDoesNotExist()
        assertNull(saved)
    }

    @Test fun storageFailureBlocksFormsAndDoesNotPretendTheSessionWasCleared() {
        storageFailure = true
        render()
        compose.onNodeWithText("Secure storage unavailable", substring = true).assertExists()
        compose.onNodeWithTag("auth-submit").assertDoesNotExist()
        compose.onNodeWithText("Clear local session").performClick()
        compose.onNodeWithText("Secure storage unavailable", substring = true).assertExists()
        assertEquals(0, calls)
    }

}
