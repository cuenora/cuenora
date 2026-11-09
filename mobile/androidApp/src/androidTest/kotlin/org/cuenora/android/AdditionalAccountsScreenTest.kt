package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.espresso.Espresso.closeSoftKeyboard
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class AdditionalAccountsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val rows = mutableListOf(AccountWithBalance(
        Account(AccountId(0, 1), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE),
        Money(0, Currency.SEK),
    ))
    private var pending: PendingAccount? = null
    private var unknown = false
    private val writes = mutableListOf<PendingAccount>()
    private val accepted = mutableSetOf<IdempotencyKey>()
    private fun render() {
        var key = 0
        val backend = object : OnboardingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun requestVerification() = RepositoryResult.Success(Unit)
            override suspend fun confirmVerification(token: String) = RepositoryResult.Success(Unit)
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun hasAccounts(owner: OwnerId) = RepositoryResult.Success(rows.isNotEmpty())
            override suspend fun create(pending: PendingAccount): RepositoryResult<Unit> {
                writes += pending
                if (accepted.add(pending.key)) rows += AccountWithBalance(
                    Account(AccountId(0, rows.size.toLong() + 1), owner, pending.command.name,
                        pending.command.type, pending.command.currency, AccountStatus.ACTIVE),
                    pending.command.openingBalance?.amount ?: Money(0, pending.command.currency),
                )
                return if (unknown) RepositoryResult.Failure(ApiFailure.Timeout) else RepositoryResult.Success(Unit)
            }
        }
        val store = object : PendingAccountStore {
            override suspend fun read(owner: OwnerId) = pending?.takeIf { it.owner == owner }
            override suspend fun save(pending: PendingAccount) { this@AdditionalAccountsScreenTest.pending = pending }
            override suspend fun clear(owner: OwnerId) { pending = null }
        }
        val flow = OnboardingSession(backend, store, { IdempotencyKey("ui-additional-${++key}") }, { "2026-11-09" })
        val browser = AccountBrowser(object : AccountReader {
            override suspend fun list(query: AccountListQuery) = RepositoryResult.Success(Page(rows.toList(), null))
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                AdditionalAccountHost(flow, { action -> scope.launch { action() } },
                    finished = { scope.launch { flow.load(); browser.refresh() } }) { add, enabled ->
                    AccountsScreen(browser, addAccount = add, addAccountEnabled = enabled,
                        submit = { action -> scope.launch { action() } })
                }
            }
        }
    }
    private fun click(text: String) = compose.onNodeWithText(text).performScrollTo().performClick()
    private fun name(value: String) {
        compose.onNodeWithTag("account-name").performScrollTo().performTextReplacement(value)
        closeSoftKeyboard()
    }
    @Test fun createsSecondReviewedAccountAndRefreshesExistingList() {
        render(); compose.onNodeWithText("Daily").assertExists()
        compose.onNodeWithTag("add-account").performScrollTo().performClick()
        name("Second account"); click("Savings"); click("EUR"); click("Review account")
        compose.onNodeWithText("Review your account").assertExists()
        compose.runOnIdle { assertTrue(writes.isEmpty()) }
        click("Create account"); compose.onNodeWithText("Your account is ready").assertExists()
        compose.onNodeWithText("Done").performClick()
        compose.onNodeWithText("Second account").performScrollTo().assertExists()
        compose.onNodeWithText("Daily").performScrollTo().assertExists()
        compose.runOnIdle {
            assertEquals(1, writes.size); assertNull(pending)
            assertEquals(AccountType.SAVINGS, writes.single().command.type)
            assertEquals(Currency.EUR, writes.single().command.currency)
        }
    }
    @Test fun appbarBackConfirmsDiscardAndKeepsExistingAccountsWithoutPosting() {
        render(); compose.onNodeWithTag("add-account").performScrollTo().performClick(); name("Draft")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("account-name").assertTextContains("Draft")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard", useUnmergedTree = true).performClick()
        compose.onNodeWithTag("additional-account-screen").assertDoesNotExist()
        compose.onNodeWithText("Daily").assertExists()
        compose.runOnIdle { assertTrue(writes.isEmpty()); assertNull(pending); assertEquals(1, rows.size) }
    }
    @Test fun reopeningUnknownCreationOffersOnlyOriginalRetryWithoutAutomaticPost() {
        unknown = true; render()
        compose.onNodeWithTag("add-account").performScrollTo().performClick(); name("Uncertain")
        click("Review account"); click("Create account")
        compose.onNodeWithText("Check the original account creation").assertExists()
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithTag("add-account").performScrollTo().performClick()
        compose.onNodeWithText("Retry original request").performScrollTo().assertExists()
        compose.runOnIdle { assertEquals(1, writes.size); assertNotNull(pending) }
        unknown = false; click("Retry original request"); compose.onNodeWithText("Done").performClick()
        compose.onNodeWithText("Uncertain").performScrollTo().assertExists()
        compose.runOnIdle {
            assertEquals(2, writes.size); assertEquals(writes[0].key, writes[1].key)
            assertEquals(2, rows.size); assertNull(pending)
        }
    }
}
