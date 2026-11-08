package org.cuenora.android

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
import org.junit.Rule
import org.junit.Test

class TransactionsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val queries = mutableListOf<TransactionQuery>()
    private var offline = false
    private var recordings = 0
    private val event = TransactionRecord(Transaction(TransactionId(0, 1), TransactionKind.TRANSFER,
        TransactionState.VOIDED, Money(12345, Currency.SEK), BusinessDate(2024, 2, 29),
        accountId = AccountId(0, 2), destinationAccountId = AccountId(0, 3), receivedAmount = Money(1099, Currency.EUR)))
    private fun render(empty: Boolean = false) {
        val reader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
                queries += query
                return if (offline) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(Page(if (empty || query.account != null) emptyList() else listOf(event), null))
            }
            override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page(listOf(
                AccountWithBalance(Account(AccountId(0, 2), OwnerId(0, 7), AccountName("Old savings"),
                    AccountType.SAVINGS, Currency.SEK, AccountStatus.ARCHIVED), Money(0, Currency.SEK))), null))
            override suspend fun categories() = RepositoryResult.Success(listOf(CatalogCategory(CategoryId(0, 4), "food", TransactionKind.EXPENSE, "Food")))
        }
        val browser = TransactionBrowser(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { TransactionsScreen(browser, { action -> scope.launch { action() } }, { recordings++ }) }
        }
    }
    @Test fun showsOneVoidedTransferWithBothExactAmountsAndPreservesRecordingEntry() {
        render()
        compose.onNodeWithText("Transfer").assertExists()
        compose.onNodeWithText("2024-02-29 · Voided").assertExists()
        compose.onNodeWithText("SEK 123.45").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("EUR 10.99").performScrollTo().assertIsDisplayed()
        compose.onAllNodesWithText("Transfer").assertCountEquals(1)
        compose.onNodeWithTag("record-from-transactions").performScrollTo().performClick()
        assertEquals(1, recordings)
    }
    @Test fun appliesServerFiltersAndCancelDoesNotReadOrChangeAppliedSelection() {
        render()
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.onNodeWithTag("history-from").performTextInput("2024-02-01")
        compose.onNodeWithTag("history-to").performTextInput("2024-02-29")
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
        compose.onNodeWithText("Reset filters").performScrollTo().performClick()
        compose.onNodeWithText("Cancel").performClick()
        compose.runOnIdle { assertEquals(2, queries.size) }
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
