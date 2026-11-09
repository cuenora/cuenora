package org.cuenora.android

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.core.view.WindowInsetsCompat
import androidx.test.espresso.Espresso.closeSoftKeyboard
import kotlinx.coroutines.CompletableDeferred
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
    private var composeView: android.view.View? = null
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val savings = daily.copy(id = AccountId(0, 10), name = AccountName("Savings"), type = AccountType.SAVINGS)
    private var pending: PendingTransaction? = null
    private var posts = 0
    private var createStarted: CompletableDeferred<Unit>? = null
    private var createGate: CompletableDeferred<Unit>? = null
    private var offline = false
    private var destinationCurrency = Currency.SEK
    private var lastCommand: TransactionCreateCommand? = null
    private var zone: String? = "Europe/Stockholm"
    private val keys = mutableListOf<IdempotencyKey>()
    private val backend = object : RecordingBackend {
        override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, zone))
        override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> {
            this@RecordingTest.zone = zone; return RepositoryResult.Success(zone)
        }
        override suspend fun accounts(owner: OwnerId) = RepositoryResult.Success(listOf(daily, savings.copy(currency = destinationCurrency)))
        override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
            posts++; keys += pending.key; lastCommand = pending.command
            createStarted?.complete(Unit)
            createGate?.await()
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
            composeView = androidx.compose.ui.platform.LocalView.current
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
    private fun fillTransfer(received: String, quote: String? = null) {
        compose.onNodeWithTag("record-TRANSFER").performScrollTo().performClick()
        enter("record-amount", "200.00"); enter("record-date", "2026-11-08")
        click("To account: Choose account")
        compose.onNodeWithText("Savings (${destinationCurrency.name})").performClick()
        enter("record-received-amount", received)
        quote?.let { enter("record-quoted-rate", it) }
    }
    @Test fun quotedCrossCurrencyTransferShowsBothActualsAndNonBlockingSignedWarning() {
        destinationCurrency = Currency.EUR; render(daily); fillTransfer("17.95", "0.09")
        // Re-selecting the current type must not discard transfer details.
        compose.onNodeWithTag("record-TRANSFER").performScrollTo().performClick()
        compose.onNodeWithTag("record-received-amount").assertTextContains("17.95")
        compose.onNodeWithTag("record-quoted-rate").assertTextContains("0.09")
        click("Review record"); assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithText("From: Daily (SEK)").assertExists()
        compose.onNodeWithText("To: Savings (EUR)").assertExists()
        compose.onNodeWithText("Rate discrepancy", substring = true).assertExists()
        compose.onNodeWithText("difference: -0.05 EUR", substring = true).assertExists()
        compose.onNodeWithText("does not count as income or spending", substring = true).assertExists()
        click("Keep editing"); compose.onNodeWithTag("record-received-amount").assertTextContains("17.95")
        click("Review record"); click("Save record")
        compose.onNodeWithText("Record saved").assertExists()
        val command = lastCommand as TransactionCreateCommand.Transfer
        assertEquals(Money(1795, Currency.EUR), command.receivedAmount)
        assertEquals("0.09", command.quotedRate); assertEquals(1, posts)
    }
    @Test fun sameCurrencyTransferRejectsUnequalAmountsAndHasNoCategoryOrAccountlessChoice() {
        render(daily); fillTransfer("199.99")
        compose.onNodeWithText("Category (optional)", substring = true).assertDoesNotExist()
        click("Review record")
        compose.onNodeWithText("same-currency amounts must match", substring = true).assertExists()
        assertEquals(0, posts)
        enter("record-received-amount", "200.00"); click("Review record")
        compose.onNodeWithText("Rate discrepancy", substring = true).assertDoesNotExist()
        click("Save record"); assertEquals(1, posts)
        assertEquals(Money(20000, Currency.SEK), (lastCommand as TransactionCreateCommand.Transfer).receivedAmount)
    }
    @Test fun unknownTransferRetriesOriginalDualAmountsAndQuote() {
        destinationCurrency = Currency.EUR; offline = true; render(daily); fillTransfer("18.01", "0.09")
        click("Review record"); click("Save record")
        compose.onNodeWithText("Resolve original record").assertExists()
        compose.onNodeWithText("difference: +0.01 EUR", substring = true).assertExists()
        val original = pending!!.encode()
        offline = false; click("Retry original request")
        compose.onNodeWithText("Record saved").assertExists()
        assertEquals(2, posts); assertEquals(keys[0], keys[1]); assertNull(pending)
        assertEquals(Money(1801, Currency.EUR), (lastCommand as TransactionCreateCommand.Transfer).receivedAmount)
        assertEquals("0.09", (PendingTransaction.decode(original).command as TransactionCreateCommand.Transfer).quotedRate)
    }
    @Test fun switchingFromTransferBackToExpenseClearsTransferOnlyFields() {
        destinationCurrency = Currency.EUR; render(daily); fillTransfer("18.00", "0.09")
        compose.onNodeWithTag("record-EXPENSE").performScrollTo().performClick()
        click("Review record"); click("Save record")
        assertTrue(lastCommand is TransactionCreateCommand.Expense); assertEquals(1, posts)
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
    @Test fun appBarBackUsesDiscardConfirmationAndPreservesEditsUntilConfirmed() {
        render(); enter("record-amount", "3.00")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("record-amount").assertTextContains("3.00")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Origin context").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }
    @Test fun appBarBackIsDisabledDuringAnInProgressSubmission() {
        val started = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        createStarted = started
        createGate = release
        render(); enter("record-amount", "3.00"); enter("record-date", "2026-11-08")
        click("Review record"); click("Save record")
        compose.waitUntil(timeoutMillis = 5_000) { started.isCompleted }
        compose.onNodeWithContentDescription("Back").assertIsNotEnabled()
        compose.onNodeWithText("Saving…").assertIsNotEnabled()
        compose.onNodeWithText("Discard changes?").assertDoesNotExist()
        release.complete(Unit)
        compose.waitUntil(timeoutMillis = 5_000) {
            compose.onAllNodesWithText("Record saved").fetchSemanticsNodes().isNotEmpty()
        }
        assertEquals(1, posts)
    }
    @Test fun cancelActionRemainsReachableAboveSystemNavigationArea() {
        render(); enter("record-amount", "3.00")
        compose.onNodeWithTag("record-cancel").performScrollTo().assertIsDisplayed()
        val windowInsets = WindowInsetsCompat.toWindowInsetsCompat(composeView!!.rootWindowInsets)
        val navigationInset = windowInsets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom /
            composeView!!.resources.displayMetrics.density
        val rootBottom = composeView!!.height / composeView!!.resources.displayMetrics.density
        val cancelBottom = compose.onNodeWithTag("record-cancel").getUnclippedBoundsInRoot().bottom
        assertTrue("Expected a visible system navigation inset", navigationInset > 0)
        assertTrue("Cancel record overlaps the system navigation area: $cancelBottom > ${rootBottom - navigationInset}dp",
            cancelBottom.value <= rootBottom - navigationInset)
        compose.onNodeWithTag("record-cancel").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Origin context").assertExists()
        assertEquals(0, posts)
    }
    @Test fun cancelActionStaysAboveSystemAndImeInsetsWhileKeyboardIsOpen() {
        render()
        compose.onNodeWithTag("record-amount").performClick().performTextInput("3.00")
        compose.waitUntil(timeoutMillis = 5_000) {
            WindowInsetsCompat.toWindowInsetsCompat(composeView!!.rootWindowInsets)
                .getInsets(WindowInsetsCompat.Type.ime()).bottom > 0
        }
        compose.onNodeWithTag("record-cancel").performScrollTo().assertIsDisplayed()
        val insets = WindowInsetsCompat.toWindowInsetsCompat(composeView!!.rootWindowInsets)
        val density = composeView!!.resources.displayMetrics.density
        val imeInset = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        val navigationInset = insets.getInsets(WindowInsetsCompat.Type.navigationBars()).bottom
        assertTrue("Expected an open software keyboard inset", imeInset > 0)
        val safeBottom = composeView!!.height / density - maxOf(imeInset, navigationInset) / density
        val cancelBottom = compose.onNodeWithTag("record-cancel").getUnclippedBoundsInRoot().bottom.value
        assertTrue("Cancel record overlaps the IME or system navigation area: $cancelBottom > $safeBottom",
            cancelBottom <= safeBottom)
        compose.onNodeWithTag("record-cancel").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Origin context").assertExists()
        assertEquals(0, posts)
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
