package org.cuenora.android

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class AccountsScreenTest {
    @get:Rule val compose = createComposeRule()
    private var offline = false
    private val requests = mutableListOf<String?>()
    private fun item(id: Long, name: String, type: AccountType, currency: Currency, status: AccountStatus, amount: Long) =
        AccountWithBalance(Account(AccountId(0, id), OwnerId(0, 7), AccountName(name), type, currency, status), Money(amount, currency))
    private val first = item(1, "Daily", AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE, Long.MAX_VALUE)
    private val second = item(2, "Old savings", AccountType.SAVINGS, Currency.EUR, AccountStatus.ARCHIVED, -1)
    private fun render(empty: Boolean = false) {
        val reader = object : AccountReader {
            override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                requests += query.cursor
                return if (offline) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(if (empty) Page(emptyList(), null)
                    else if (query.cursor == null) Page(listOf(first), "next") else Page(listOf(second), null))
            }
        }
        val browser = AccountBrowser(reader)
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme { CuenoraApp(accountsContent = { AccountsScreen(browser) { action -> scope.launch { action() } } }) }
        }
        if (compose.onAllNodesWithContentDescription("Open destinations").fetchSemanticsNodes().isNotEmpty())
            compose.onNodeWithContentDescription("Open destinations").performClick()
        compose.onNodeWithTag("nav-Accounts").performClick()
    }
    @Test fun listAndDetailShowExactMetadataAndBackKeepsLoadedPages() {
        render()
        compose.onNodeWithText("Daily").assertIsDisplayed()
        compose.onNodeWithText("SEK 92233720368547758.07").assertExists()
        compose.onNodeWithText("Load more accounts").performScrollTo().performClick()
        compose.onNodeWithText("Old savings").performScrollTo().performClick()
        compose.onNodeWithText("Account details").assertExists()
        compose.onNodeWithText("Type: Savings").assertExists()
        compose.onNodeWithText("Status: Archived").assertExists()
        compose.onNodeWithText("Currency: EUR").assertExists()
        compose.onNodeWithText("EUR -0.01").assertExists()
        compose.onNodeWithText("Back to accounts").performClick()
        compose.onNodeWithText("Old savings").assertExists()
        compose.onNodeWithText("Old savings").performScrollTo().performClick()
        androidx.test.espresso.Espresso.pressBack()
        compose.onNodeWithText("Account details").assertDoesNotExist()
        compose.onNodeWithText("Old savings").assertExists()
        assertEquals(listOf(null, "next"), requests)
    }
    @Test fun failedReadOffersRetryWithoutZeroOrEmptyState() {
        offline = true; render()
        compose.onNodeWithText("Accounts could not be loaded", substring = true).assertExists()
        compose.onNodeWithText("No accounts yet").assertDoesNotExist()
        compose.onNodeWithText("SEK 0.00").assertDoesNotExist()
        offline = false; compose.onNodeWithText("Try again").performScrollTo().performClick()
        compose.onNodeWithText("Daily").assertExists()
        assertEquals(listOf(null, null), requests)
    }
    @Test fun confirmedEmptyIsNotFailureAndProvidesOnboardingDirection() {
        render(empty = true)
        compose.onNodeWithText("No accounts yet").assertExists()
        compose.onNodeWithText("Accounts could not be loaded", substring = true).assertDoesNotExist()
        compose.onNodeWithText("Set up your first account from Home.").assertExists()
    }
    @Test fun nextPageFailureKeepsExistingRowsAndRetriesOriginalCursor() {
        render(); compose.onNodeWithText("Daily").assertIsDisplayed(); offline = true
        compose.onNodeWithText("Load more accounts").performScrollTo().performClick()
        compose.onNodeWithText("More accounts could not be loaded", substring = true).assertExists()
        compose.onNodeWithText("Daily").assertExists()
        offline = false
        compose.onNodeWithText("Retry more accounts").performScrollTo().performClick()
        compose.onNodeWithText("Old savings").assertExists()
        assertEquals(listOf(null, "next", "next"), requests)
    }
}
