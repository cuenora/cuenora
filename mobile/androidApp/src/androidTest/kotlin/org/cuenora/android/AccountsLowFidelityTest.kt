package org.cuenora.android

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

/** PDaOP overview structure with synthetic balances and existing navigation only. */
class AccountsLowFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val addCalls = AtomicInteger()
    private val reads = AtomicInteger()
    private fun row(id: Long, name: String, type: AccountType, currency: Currency, amount: Long) =
        AccountWithBalance(Account(AccountId(0, id), OwnerId(0, 7), AccountName(name), type, currency, AccountStatus.ACTIVE),
            Money(amount, currency))
    private val rows = listOf(
        row(1, "Account A", AccountType.NORMAL, Currency.SEK, 250000),
        row(2, "Account B", AccountType.NORMAL, Currency.SEK, 50000),
        row(3, "Savings S", AccountType.SAVINGS, Currency.SEK, 100000),
        row(4, "Euro account", AccountType.NORMAL, Currency.EUR, 11800),
    )

    private fun render(narrow: Boolean = false) {
        val browser = AccountBrowser(object : AccountReader {
            override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                reads.incrementAndGet()
                return RepositoryResult.Success(Page(rows, null))
            }
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("accounts-preview")) {
                    CuenoraTheme(darkTheme = narrow) {
                        CuenoraApp(accountsContent = {
                            AccountsScreen(browser, addAccount = { addCalls.incrementAndGet() },
                                submit = { action -> scope.launch { action() } })
                        })
                    }
                }
            }
        }
        if (compose.onAllNodesWithContentDescription("Choose destination").fetchSemanticsNodes().isNotEmpty())
            compose.onNodeWithContentDescription("Choose destination").performClick()
        compose.onNodeWithTag("nav-Accounts").performClick()
    }

    @Test fun overviewLeadsWithCreationAndUsesCurrencyTotalsAndOpenAccountRows() {
        render()
        compose.onNodeWithText("Your accounts").assertDoesNotExist()
        val add = compose.onNodeWithTag("add-account").fetchSemanticsNode().boundsInRoot
        val total = compose.onNodeWithTag("accounts-total-SEK").fetchSemanticsNode().boundsInRoot
        assertTrue("Creation should precede the currency section", add.bottom < total.top)
        compose.onNodeWithText("SEK total").assertExists()
        compose.onNodeWithText("SEK 4000.00").assertExists()
        compose.onNodeWithTag("accounts-open-row-${rows.first().account.id}").assertExists()
        compose.onNodeWithTag("accounts-row-divider-${rows.first().account.id}").assertExists()
        capture("accounts-light-overview")
        compose.onNodeWithTag("add-account").performClick()
        assertEquals(1, addCalls.get())
        val accountList = compose.onNodeWithTag("account-list")
        accountList.performScrollToKey(AccountId(0, 3).toString())
        compose.onNodeWithText("Savings S").assertIsDisplayed()
        compose.onNodeWithText("Included in the SEK total above.").assertExists()
        capture("accounts-light-savings")
        accountList.performScrollToKey(rows.first().account.id.toString())
        compose.onNodeWithTag("accounts-open-row-${rows.first().account.id}").performClick()
        compose.onNodeWithText("Account details").assertExists()
        compose.onNodeWithText("Normal account").assertExists()
        compose.onNodeWithContentDescription("Back to accounts").performClick()
        assertEquals("Presentation and detail navigation reuse the loaded page", 1, reads.get())
    }

    @Test fun narrowLargeTextKeepsBalancesAndNavigationReadable() {
        render(narrow = true)
        compose.onNodeWithTag("accounts-total-SEK").assertExists()
        compose.onNodeWithText("SEK 2500.00").performScrollTo().assertIsDisplayed()
        val amount = compose.onNodeWithText("SEK 2500.00").fetchSemanticsNode().boundsInRoot
        val preview = compose.onNodeWithTag("accounts-preview").fetchSemanticsNode().boundsInRoot
        assertTrue("Exact amount must stay within the viewport", amount.left >= preview.left && amount.right <= preview.right)
        capture("accounts-dark-large-text")
        compose.onNodeWithText("Account A").performScrollTo().performClick()
        compose.onNodeWithText("Currency: SEK").assertExists()
        compose.onNodeWithContentDescription("Back to accounts").performScrollTo().performClick()
        compose.onNodeWithText("Refresh accounts").performScrollTo().performClick()
        compose.waitForIdle()
        assertEquals(2, reads.get())
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("accounts-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
