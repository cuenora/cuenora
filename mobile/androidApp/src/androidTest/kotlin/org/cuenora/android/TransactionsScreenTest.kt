package org.cuenora.android

import android.graphics.Bitmap
import androidx.compose.material3.Surface
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Before
import org.junit.After
import java.util.Locale
import org.junit.Rule
import org.junit.Test

class TransactionsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val originalLocale = Locale.getDefault()
    @Before fun useDeterministicEnglishDates() { Locale.setDefault(Locale.UK) }
    @After fun restoreLocale() { Locale.setDefault(originalLocale) }
    private val queries = mutableListOf<TransactionQuery>()
    private var offline = false
    private var recordings = 0
    private val event = TransactionRecord(Transaction(TransactionId(0, 1), TransactionKind.TRANSFER,
        TransactionState.VOIDED, Money(12345, Currency.SEK), BusinessDate(2024, 2, 29),
        accountId = AccountId(0, 2), destinationAccountId = AccountId(0, 3), receivedAmount = Money(1099, Currency.EUR)))
    private fun render(empty: Boolean = false, narrow: Boolean = false, signedRows: Boolean = false) {
        val signedEvents = listOf(
            TransactionRecord(Transaction(TransactionId(0, 8), TransactionKind.EXPENSE, TransactionState.POSTED,
                Money(30000, Currency.SEK), BusinessDate(2024, 2, 28), description = "Groceries")),
            TransactionRecord(Transaction(TransactionId(0, 9), TransactionKind.INCOME, TransactionState.POSTED,
                Money(200000, Currency.SEK), BusinessDate(2024, 2, 28), description = "Work income")),
            TransactionRecord(Transaction(TransactionId(0, 10), TransactionKind.OPENING_BALANCE, TransactionState.POSTED,
                Money(-10000, Currency.SEK), BusinessDate(2024, 2, 27), accountId = AccountId(0, 2))),
        )
        val reader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
                queries += query
                return if (offline) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(Page(if (empty || query.account != null) emptyList() else if (signedRows) signedEvents else listOf(event), null))
            }
            override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page(listOf(
                AccountWithBalance(Account(AccountId(0, 2), OwnerId(0, 7), AccountName("Old savings"),
                    AccountType.SAVINGS, Currency.SEK, AccountStatus.ARCHIVED), Money(0, Currency.SEK))), null))
            override suspend fun categories() = RepositoryResult.Success(listOf(CatalogCategory(CategoryId(0, 4), "food", TransactionKind.EXPENSE, "Food")))
        }
        val browser = TransactionBrowser(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("transactions-preview")) {
                    CuenoraTheme(darkTheme = narrow) {
                        Surface(Modifier.fillMaxSize()) {
                            TransactionsScreen(browser, { action -> scope.launch { action() } }, { recordings++ })
                        }
                    }
                }
            }
        }
    }
    @Test fun overviewUsesOpenDatedRowsAndLeadingActionsWithoutRedundantSubtitle() {
        render()
        compose.onNodeWithText("Recorded activity").assertDoesNotExist()
        val primary = compose.onNodeWithTag("record-from-transactions").fetchSemanticsNode().boundsInRoot
        val filters = compose.onNodeWithTag("transactions-filters").fetchSemanticsNode().boundsInRoot
        assertTrue(primary.bottom <= filters.top)
        assertEquals(primary.width, filters.width, 1f)
        capture("transactions-light")
        compose.onNodeWithTag("transactions-filters").assertExists()
        compose.onNodeWithText("Dates: any to any · All accounts · All categories").assertExists()
        compose.onNodeWithText("29 February 2024").performScrollTo().assertExists()
        compose.onNodeWithTag("transaction-divider-${event.transaction.id}").performScrollTo().assertExists()
        compose.onNodeWithText("Only recorded operations appear here. Upcoming payments remain in Recurrence.")
            .performScrollTo().assertExists()
        compose.onNodeWithText("Refresh transactions").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(2, queries.size) }
    }
    @Test fun narrowLargeTextKeepsExactTransferAmountsAndActionsReachable() {
        render(narrow = true)
        capture("transactions-dark-large-text")
        compose.onNodeWithText("SEK 123.45").performScrollTo().assertIsDisplayed()
        val amount = compose.onNodeWithText("SEK 123.45").fetchSemanticsNode().boundsInRoot
        val viewport = compose.onNodeWithTag("transactions-preview").fetchSemanticsNode().boundsInRoot
        assertTrue(amount.left >= viewport.left && amount.right <= viewport.right)
        compose.onNodeWithText("EUR 10.99").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-from-transactions").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(1, recordings); assertEquals(1, queries.size) }
        compose.onNodeWithTag("transactions-filters").performScrollTo().performClick()
        compose.onNodeWithText("Cancel").performClick()
        compose.runOnIdle { assertEquals(1, queries.size) }
    }
    @Test fun signedRowsKeepDescriptionsBusinessDatesAndExactAmountsWithoutTotals() {
        render(signedRows = true)
        compose.onAllNodesWithText("28 February 2024").assertCountEquals(1)
        compose.onNodeWithText("Groceries").performScrollTo().assertExists()
        compose.onNodeWithText("−SEK 300.00").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Work income").performScrollTo().assertExists()
        compose.onNodeWithText("+SEK 2000.00").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("−SEK 100.00").performScrollTo().assertIsDisplayed()
        compose.runOnIdle { assertEquals(1, queries.size) }
    }
    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("transactions-preview").captureToImage().asAndroidBitmap()
                .compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
    @Test fun showsOneVoidedTransferWithBothExactAmountsAndPreservesRecordingEntry() {
        render()
        compose.onNodeWithText("Transfer").assertExists()
        compose.onNodeWithText("29 February 2024").assertExists()
        compose.onNodeWithText("Voided").performScrollTo().assertExists()
        compose.onNodeWithText("SEK 123.45").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("EUR 10.99").performScrollTo().assertIsDisplayed()
        compose.onAllNodesWithText("Transfer").assertCountEquals(1)
        compose.onNodeWithTag("record-from-transactions").performScrollTo().performClick()
        assertEquals(1, recordings)
    }
    @Test fun appliesServerFiltersAndCancelDoesNotReadOrChangeAppliedSelection() {
        render()
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.selectNativeDate("history-from", 2024, 2, 1)
        compose.selectNativeDate("history-to", 2024, 2, 29)
        compose.onNodeWithText("No account assigned").performScrollTo().performClick()
        compose.onNodeWithText("Food · Expense").performScrollTo().performClick()
        compose.onNodeWithText("Apply filters").performClick()
        compose.onNodeWithText("No matching transactions").performScrollTo().assertExists()
        compose.runOnIdle {
            assertEquals(BusinessDate(2024, 2, 1), queries.last().dateFrom)
            assertEquals(BusinessDate(2024, 2, 29), queries.last().dateTo)
            assertEquals(CategoryId(0, 4), queries.last().categoryId)
            assertEquals(TransactionAccountFilter.Accountless, queries.last().account)
            assertNull(queries.last().cursor)
        }
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.clearNativeDate("history-from")
        compose.onNodeWithText("Apply filters").performClick()
        compose.runOnIdle {
            assertNull(queries.last().dateFrom)
            assertEquals(BusinessDate(2024, 2, 29), queries.last().dateTo)
        }
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.onNodeWithText("Reset filters").performScrollTo().performClick()
        compose.onNodeWithText("Cancel").performClick()
        compose.runOnIdle { assertEquals(3, queries.size) }
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.onNodeWithText("Selected: No account assigned").performScrollTo().assertExists()
        compose.onNodeWithText("Reset filters").performScrollTo().performClick()
        compose.onNodeWithText("Apply filters").performClick()
        compose.onNodeWithText("Transfer").performScrollTo().assertExists()
        compose.runOnIdle { assertNull(queries.last().account); assertNull(queries.last().dateFrom) }
    }
    @Test fun offlineIsNotEmptyAndRetryLoadsConfirmedEmpty() {
        offline = true; render(empty = true)
        compose.onNodeWithText("Transactions unavailable", substring = true).assertExists()
        compose.onNodeWithText("No recorded transactions yet").assertDoesNotExist()
        offline = false
        compose.onNodeWithText("Try again").performScrollTo().performClick()
        compose.onNodeWithText("No recorded transactions yet").performScrollTo().assertExists()
    }
}
