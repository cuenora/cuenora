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

/** Synthetic detail previews pin low-fidelity hierarchy without adding financial behavior. */
class AccountDetailLowFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val normal = account(41, "Everyday account", AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE, 123456)
    private val savings = account(42, "Long-term savings", AccountType.SAVINGS, Currency.EUR, AccountStatus.ACTIVE, 123456789)

    private fun account(id: Long, name: String, type: AccountType, currency: Currency, status: AccountStatus, minor: Long) =
        AccountWithBalance(Account(AccountId(0, id), OwnerId(0, 7), AccountName(name), type, currency, status), Money(minor, currency))

    private fun render(item: AccountWithBalance, narrow: Boolean = false, record: ((Account) -> Unit)? = null,
                       savingsAction: ((Account) -> Unit)? = null): Pair<AccountBrowser, IntArray> {
        val reads = intArrayOf(0)
        val browser = AccountBrowser(object : AccountReader {
            override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                reads[0]++
                return RepositoryResult.Success(Page(listOf(item), null))
            }
        })
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("account-detail-preview")) {
                    CuenoraTheme(darkTheme = narrow) {
                        CuenoraApp(accountsContent = {
                            AccountsScreen(browser, recordActivity = record, savingsActivity = savingsAction,
                                submit = { action -> scope.launch { action() } })
                        })
                    }
                }
            }
        }
        if (compose.onAllNodesWithContentDescription("Choose destination").fetchSemanticsNodes().isNotEmpty())
            compose.onNodeWithContentDescription("Choose destination").performClick()
        compose.onNodeWithTag("nav-Accounts").performClick()
        compose.onNodeWithTag("accounts-open-row-${item.account.id}").performClick()
        return browser to reads
    }

    @Test fun normalDetailOrdersBalanceBeforeRecordingAndBackReusesLoadedAccount() {
        var recorded: Account? = null
        val (browser, reads) = render(normal, record = { recorded = it })
        compose.onNodeWithText("Everyday account").assertIsDisplayed()
        compose.onNodeWithText("Normal account").assertExists()
        compose.onNodeWithText("Currency: SEK").assertExists()
        compose.onNodeWithText("Status: Active").assertExists()
        compose.onNodeWithText("SEK 1234.56").assertExists()
        val balance = compose.onNodeWithText("SEK 1234.56").fetchSemanticsNode().boundsInRoot
        val record = compose.onNodeWithTag("record-from-account").fetchSemanticsNode().boundsInRoot
        assertTrue("The posted balance must precede the recording action", balance.bottom < record.top)
        capture("account-detail-light")

        compose.onNodeWithTag("record-from-account").performClick()
        assertSame("Recording reuses the selected account", normal.account, recorded)
        compose.onNodeWithText("Back to accounts").performClick()
        compose.onNodeWithText("Everyday account").assertExists()
        assertEquals("Back must not reload the already loaded account page", 1, reads[0])
        assertTrue("The browser remains on the same loaded page", browser.state.value.loaded)
    }

    @Test fun savingsDetailKeepsExactCurrencyAndReflowsInNarrowDarkLargeText() {
        var selected: Account? = null
        render(savings, narrow = true, savingsAction = { selected = it })
        compose.onNodeWithText("Long-term savings").assertIsDisplayed()
        compose.onNodeWithText("Savings account").assertExists()
        compose.onNodeWithText("Currency: EUR").assertExists()
        compose.onNodeWithText("Status: Active").assertExists()
        val amountNode = compose.onNodeWithText("EUR 1234567.89")
        amountNode.assertIsDisplayed()
        val amount = amountNode.fetchSemanticsNode().boundsInRoot
        val viewport = compose.onNodeWithTag("account-detail-preview").fetchSemanticsNode().boundsInRoot
        assertTrue("The full exact amount must stay within the narrow viewport", amount.left >= viewport.left && amount.right <= viewport.right)
        compose.onNodeWithTag("savings-from-account").assertIsDisplayed().performClick()
        assertSame("Savings action reuses the selected account", savings.account, selected)
        capture("account-detail-dark-large-text")
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("account-detail-preview").captureToImage().asAndroidBitmap()
                .compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
