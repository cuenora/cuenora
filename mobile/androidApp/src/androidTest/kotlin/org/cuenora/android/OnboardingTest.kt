package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.espresso.Espresso.closeSoftKeyboard
import kotlinx.coroutines.launch
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.android.ui.OnboardingRoot
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class OnboardingTest {
    @get:Rule val compose = createComposeRule()
    private var verified = false
    private var existing = false
    private var offline = false
    private var createAttempts = 0
    private val keys = mutableListOf<IdempotencyKey>()
    private var saved: PendingAccount? = null
    private var zone: String? = null
    private val backend = object : OnboardingBackend {
        override suspend fun profile(): RepositoryResult<OnboardingProfile> = if (offline)
            RepositoryResult.Failure(ApiFailure.Network) else RepositoryResult.Success(OnboardingProfile(OwnerId(0, 7), verified, zone))
        override suspend fun requestVerification() = RepositoryResult.Success(Unit)
        override suspend fun confirmVerification(token: String): RepositoryResult<Unit> {
            verified = true; return RepositoryResult.Success(Unit)
        }
        override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> {
            this@OnboardingTest.zone = zone; return RepositoryResult.Success(zone)
        }
        override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(existing)
        override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
            createAttempts++; keys += pending.key
            return if (createAttempts == 1) RepositoryResult.Failure(ApiFailure.Timeout) else RepositoryResult.Success(Unit)
        }
    }
    private val store = object : PendingAccountStore {
        override suspend fun read(owner: OwnerId) = saved
        override suspend fun save(pending: PendingAccount) { saved = pending }
        override suspend fun clear(owner: OwnerId) { saved = null }
    }
    private fun render() {
        val flow = OnboardingSession(backend, store, { IdempotencyKey("synthetic-ui-key") }, { "2026-11-08" })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { OnboardingRoot(flow) { action -> scope.launch { action() } } }
        }
    }
    private fun click(text: String) { compose.onNodeWithText(text).performScrollTo().performClick() }
    private fun enter(tag: String, value: String) {
        compose.onNodeWithTag(tag).performScrollTo().performTextReplacement(value)
        closeSoftKeyboard()
    }
    @Test fun verificationZoneReviewAndUncertainRetryKeepOneLogicalCreation() {
        render()
        click("Request verification email")
        compose.onNodeWithText("The request was accepted.", substring = true).assertExists()
        assertFalse(verified)
        enter("verification-token", "ev1_" + "a".repeat(64)); click("Confirm token")
        click("Add first account")
        enter("account-name", "Daily")
        compose.onNodeWithTag("account-opening-enabled").performScrollTo().performClick()
        enter("account-zone", "Europe/Stockholm"); click("Confirm time zone")
        enter("account-opening-amount", "12.34")
        click("Review account")
        compose.onNodeWithText("Review your account").assertExists()
        assertEquals(0, createAttempts)
        click("Create account")
        compose.onNodeWithText("Check the original account creation").assertExists()
        assertNotNull(saved)
        click("Retry original request")
        compose.onNodeWithText("Your account is ready").assertExists()
        assertEquals(2, createAttempts); assertEquals(keys[0], keys[1]); assertNull(saved)
    }
    @Test fun cancelOffersKeepEditingAndDiscardsWithoutFinancialWrites() {
        verified = true; zone = "Europe/Stockholm"; render()
        click("Add first account"); enter("account-name", "Unsaved")
        click("Cancel setup")
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("account-name").assertTextContains("Unsaved")
        click("Cancel setup"); compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Your first account").assertExists()
        assertEquals(0, createAttempts); assertNull(saved)
    }
    @Test fun existingOwnerIsNotForcedThroughFirstAccountSetup() {
        verified = true; existing = true; render()
        compose.onNodeWithText("Welcome back").assertExists()
        compose.onNodeWithText("Add first account").assertDoesNotExist()
        assertEquals(0, createAttempts)
    }
    @Test fun failedLoadIsNeverRenderedAsAnEmptyAccountList() {
        offline = true; render()
        compose.onNodeWithText("Setup could not be loaded").assertExists()
        compose.onNodeWithText("Add first account").assertDoesNotExist()
        offline = false; verified = true; click("Try again")
        compose.onNodeWithText("Your first account").assertExists()
    }
}
