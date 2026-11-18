package org.cuenora.android

import android.graphics.Bitmap
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.espresso.Espresso.closeSoftKeyboard
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.recording.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.TransactionForm
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class TransferFormFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val euro = daily.copy(id = AccountId(0, 10), name = AccountName("Euro savings"), type = AccountType.SAVINGS, currency = Currency.EUR)
    private val archived = daily.copy(id = AccountId(0, 11), name = AccountName("Archived"), status = AccountStatus.ARCHIVED)
    private var pending: PendingTransaction? = null
    private var posts = 0
    private var command: TransactionCreateCommand? = null

    private fun render(narrow: Boolean = false, dark: Boolean = false, sameCurrency: Boolean = false,
        unavailable: Boolean = false, sent: String = "200.00", received: String = "18.00", quote: String? = null) {
        val destination = if (sameCurrency) euro.copy(currency = Currency.SEK) else euro
        val backend = object : RecordingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>> =
                if (unavailable) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(listOf(daily, destination, archived))
            override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
                posts++; command = pending.command
                return RepositoryResult.Success(Unit)
            }
        }
        val store = object : PendingTransactionStore {
            override suspend fun read(owner: OwnerId) = pending
            override suspend fun save(pending: PendingTransaction) { this@TransferFormFidelityTest.pending = pending }
            override suspend fun clear(owner: OwnerId) { pending = null }
        }
        val flow = RecordingSession(backend, store, { RepositoryResult.Success(emptyList()) },
            { IdempotencyKey("transfer-preview-key") }, { "2026-11-18" })
        runBlocking { flow.begin() }
        flow.update(TransactionForm(TransactionKind.TRANSFER, sent, Currency.SEK, "2026-10-20",
            accountId = if (unavailable) null else daily.id, destinationAccountId = if (unavailable) null else destination.id,
            receivedAmount = received, receivedCurrency = if (unavailable) null else destination.currency,
            quotedRate = quote, description = "Recorded transfer"))
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("transfer-preview")) {
                    CuenoraTheme(darkTheme = dark) {
                        Surface(Modifier.fillMaxSize()) {
                            RecordingScreen(flow, { action -> scope.launch { action() } }, flow::cancel)
                        }
                    }
                }
            }
        }
    }

    @Test fun transferFollowsPenOrderAndPreviewsExactMovementsBeforeExplicitSave() {
        render()
        compose.onNodeWithText("Record transfer").assertExists()
        compose.onNodeWithText("Transactions").assertExists()
        compose.onNodeWithText("Record a transfer you made outside Cuenora.").assertExists()
        val tags = listOf("transfer-from-account", "transfer-to-account", "record-amount", "record-received-amount",
            "record-quoted-rate", "record-date", "record-description", "transfer-movements", "record-review")
        tags.map { compose.onNodeWithTag(it).getUnclippedBoundsInRoot() }.zipWithNext().forEach { (a, b) ->
            assertTrue(a.bottom <= b.top)
        }
        capture("transfer-light-top")
        compose.onNodeWithText("−SEK 200.00").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("+EUR 18.00").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("One transfer. No income or spending.").assertExists()
        compose.onNodeWithText("Review transfer").performScrollTo().assertIsDisplayed()
        capture("transfer-light-actions")
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-review").performClick()
        compose.onNodeWithText("Review your record").assertExists()
        compose.onNodeWithText("SEK 200.00").assertExists()
        compose.onNodeWithText("EUR 18.00").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        val transfer = command as TransactionCreateCommand.Transfer
        assertEquals(daily.id, transfer.accountId); assertEquals(euro.id, transfer.destinationAccountId)
        assertEquals(Money(20000, Currency.SEK), transfer.amount)
        assertEquals(Money(1800, Currency.EUR), transfer.receivedAmount)
        assertEquals(BusinessDate(2026, 10, 20), transfer.occurredOn)
        assertEquals("Recorded transfer", transfer.description); assertNull(transfer.quotedRate)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun quotedRateNeverReplacesActualAmountsAndCancelRetainsDraft() {
        render(dark = true, received = "17.95", quote = "0.09")
        capture("transfer-dark-top")
        compose.onNodeWithText("+EUR 17.95").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("EUR received per 1 SEK sent. No rate is required when both amounts are entered.")
            .performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Rate discrepancy", substring = true).assertExists()
        compose.onNodeWithText("difference: -0.05 EUR", substring = true).assertExists()
        compose.onNodeWithText("Keep editing").performScrollTo().performClick()
        compose.onNodeWithTag("record-received-amount").assertTextContains("17.95")
        compose.onNodeWithTag("record-cancel").performScrollTo().performClick()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("record-quoted-rate").assertTextContains("0.09")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun missingAccountsAndInvalidAmountsNeverFabricateMovementValues() {
        render(unavailable = true, sent = "0", received = "1,234.56")
        compose.onNodeWithText("−SEK 0.00").assertDoesNotExist()
        compose.onNodeWithText("+EUR 0.00").assertDoesNotExist()
        compose.onNodeWithText("Choose a source account and enter a positive sent amount.").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Choose a destination account and enter a positive received amount.").assertExists()
        compose.onNodeWithText("Accounts could not be loaded. Cancel and retry before recording a transfer.")
            .performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your record").assertDoesNotExist()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun selectedAccountsNeverTurnInvalidOrOutOfRangeDraftsIntoMovementValues() {
        render(sent = "92233720368547758.08", received = "-1")
        compose.onNodeWithText("Choose a source account and enter a positive sent amount.").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Choose a destination account and enter a positive received amount.").assertExists()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your record").assertDoesNotExist()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun accountSelectorsKeepActualsAndRejectDuplicateAccountsWithoutPosting() {
        render(quote = "0.09")
        compose.onNodeWithTag("transfer-from-account").performScrollTo().performClick()
        compose.onNodeWithText("Archived (SEK)").assertDoesNotExist()
        compose.onNode(hasText("Euro savings (EUR)") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Choose a distinct active account with the matching currency.").assertExists()
        compose.onNodeWithText("Review your record").assertDoesNotExist()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("transfer-from-account").performScrollTo().performClick()
        compose.onNodeWithText("Daily (SEK)").performClick()
        compose.onNodeWithTag("transfer-to-account").performScrollTo().performClick()
        compose.onNode(hasText("Daily (SEK)") and hasAnyAncestor(isPopup()), useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithText("Archived (SEK)").assertDoesNotExist()
        compose.onNode(hasText("Euro savings (EUR)") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("record-received-amount").assertTextContains("18.00")
        compose.onNodeWithTag("record-quoted-rate").assertTextContains("0.09")
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your record").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun sameCurrencyMismatchStaysRejectedWithoutChangingEnteredAmounts() {
        render(sameCurrency = true)
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("same-currency amounts must match", substring = true).assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("record-received-amount").performScrollTo().performTextReplacement("200,00")
        closeSoftKeyboard()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your record").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        val transfer = command as TransactionCreateCommand.Transfer
        assertEquals(transfer.amount, transfer.receivedAmount); assertEquals(Money(20000, Currency.SEK), transfer.amount)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun narrowLargeTextKeepsBothAccountsAmountsAndActionsReachable() {
        render(dark = true, narrow = true, received = "17.95", quote = "0.09")
        capture("transfer-dark-large-top")
        for (tag in listOf("transfer-from-account", "transfer-to-account", "record-amount", "record-received-amount",
            "record-quoted-rate", "record-date", "record-description", "record-review", "record-cancel")) {
            val bounds = compose.onNodeWithTag(tag).performScrollTo().assertIsDisplayed().getUnclippedBoundsInRoot()
            val viewport = compose.onNodeWithTag("transfer-preview").getUnclippedBoundsInRoot()
            assertTrue("$tag must fit the viewport", bounds.left >= viewport.left && bounds.right <= viewport.right)
        }
        capture("transfer-dark-large-actions")
        compose.onNodeWithTag("record-TRANSFER").performScrollTo().performClick()
        compose.onNodeWithTag("record-received-amount").assertTextContains("17.95")
        compose.onNodeWithTag("record-quoted-rate").assertTextContains("0.09")
        compose.onNodeWithTag("record-INCOME").performScrollTo().performClick()
        compose.onNodeWithText("Review record").performScrollTo().assertExists()
        compose.onNodeWithTag("record-EXPENSE").performScrollTo().performClick()
        compose.onNodeWithText("Record expense").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("transfer-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
