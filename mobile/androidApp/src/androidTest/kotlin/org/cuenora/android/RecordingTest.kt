package org.cuenora.android

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.espresso.Espresso.closeSoftKeyboard
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.recording.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class RecordingTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val savings = daily.copy(id = AccountId(0, 10), name = AccountName("Savings"), type = AccountType.SAVINGS)
    private var pending: PendingTransaction? = null
    private var posts = 0
    private var offline = false
    private var zone: String? = "Europe/Stockholm"
    private val keys = mutableListOf<IdempotencyKey>()
    private val backend = object : RecordingBackend {
        override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, zone))
        override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> {
            this@RecordingTest.zone = zone; return RepositoryResult.Success(zone)
        }
        override suspend fun accounts(owner: OwnerId) = RepositoryResult.Success(listOf(daily, savings))
        override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
            posts++; keys += pending.key
            return if (offline) RepositoryResult.Failure(ApiFailure.Timeout) else RepositoryResult.Success(Unit)
        }
    }
    private val store = object : PendingTransactionStore {
        override suspend fun read(owner: OwnerId) = pending
        override suspend fun save(pending: PendingTransaction) { this@RecordingTest.pending = pending }
        override suspend fun clear(owner: OwnerId) { pending = null }
    }
    private fun render(account: Account? = null) {
        val flow = RecordingSession(backend, store, { RepositoryResult.Success(emptyList()) },
            { IdempotencyKey("ui-original-key") }, { "2026-11-08" })
        compose.setContent {
            val scope = rememberCoroutineScope()
            val state by flow.state.collectAsState()
            LaunchedEffect(Unit) { flow.begin(account) }
            CuenoraTheme {
                if (state.stage == RecordingStage.Closed) androidx.compose.material3.Text("Origin context")
                else RecordingScreen(flow, { action -> scope.launch { action() } }, flow::cancel)
            }
        }
    }
    private fun click(text: String) = compose.onNodeWithText(text).performScrollTo().performClick()
    private fun enter(tag: String, text: String) {
        compose.onNodeWithTag(tag).performScrollTo().performTextReplacement(text); closeSoftKeyboard()
    }
    @Test fun accountlessExpenseReviewsAndSavesOnlyOnExplicitConfirmation() {
        render(); enter("record-amount", "12.34"); enter("record-date", "2026-11-08")
        click("Review record"); assertEquals(0, posts)
        compose.onNodeWithText("Included in reports; no account balance changes.").assertExists()
        click("Save record"); compose.onNodeWithText("Record saved").assertExists()
        assertEquals(1, posts); assertNull(pending)
        click("Done"); compose.onNodeWithText("Origin context").assertExists()
    }
    @Test fun accountDetailPreselectionRetainsCurrencyAndRejectsSavingsIncome() {
        render(savings); enter("record-amount", "12.34"); enter("record-date", "2026-11-08")
        compose.onNodeWithTag("record-INCOME").performScrollTo().performClick()
        click("Review record")
        compose.onNodeWithText("Choose an eligible same-currency account", substring = true).assertExists()
        assertEquals(0, posts)
        click("Account: Savings"); compose.onNodeWithText("No account assigned").performClick()
        click("Review record"); click("Save record"); assertEquals(1, posts)
    }
    @Test fun uncertainWriteRetriesSameOperationAndNeverReplacesIt() {
        offline = true; render(daily); enter("record-amount", "20.00"); enter("record-date", "2026-11-08")
        click("Review record"); click("Save record")
        compose.onNodeWithText("Resolve original record").assertExists(); assertNotNull(pending)
        offline = false; click("Retry original request")
        assertEquals(2, posts); assertEquals(keys[0], keys[1]); assertNull(pending)
    }
    @Test fun cancellationRetainsEditingThenDiscardsWithoutWrite() {
        render(); enter("record-amount", "3.00"); click("Cancel record")
        compose.onNodeWithText("Keep editing").performClick(); compose.onNodeWithTag("record-amount").assertTextContains("3.00")
        click("Cancel record"); compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Origin context").assertExists(); assertEquals(0, posts); assertNull(pending)
    }
    @Test fun accountlessOwnerCanConfirmZoneWithoutCreatingAccount() {
        zone = null; render(); enter("record-zone", "Europe/Stockholm"); click("Confirm time zone")
        compose.onNodeWithTag("record-amount").assertExists(); assertEquals("Europe/Stockholm", zone); assertEquals(0, posts)
    }
    @Test fun allNavigationEntryPointsReachCallbackAndRetainAccountContext() {
        var entry = ""
        val browser = AccountBrowser(object : org.cuenora.shared.accounts.AccountReader {
            override suspend fun list(query: AccountListQuery) = RepositoryResult.Success(Page(listOf(AccountWithBalance(daily, Money(0, Currency.SEK))), null))
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                CuenoraApp(accountsContent = { AccountsScreen(browser, recordActivity = { entry = it.name.value }, submit = { action -> scope.launch { action() } }) },
                    transactionsContent = { RecordingEntry { entry = "Transactions" } }) {
                    androidx.compose.material3.TextButton(onClick = { entry = "Home" }, modifier = androidx.compose.ui.Modifier.testTag("home-entry")) {
                        androidx.compose.material3.Text("Record income or expense")
                    }
                }
            }
        }
        compose.onNodeWithTag("home-entry").performClick(); assertEquals("Home", entry)
        compose.onNodeWithTag("nav-Transactions").performClick()
        compose.onNodeWithTag("record-from-transactions").performClick(); assertEquals("Transactions", entry)
        compose.onNodeWithTag("nav-Accounts").performClick(); click("Daily")
        compose.onNodeWithTag("record-from-account").performScrollTo().performClick(); assertEquals("Daily", entry)
    }
}
