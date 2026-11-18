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
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.recording.*
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class ExpenseFormFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val category = CatalogCategory(CategoryId(0, 4), "transport", TransactionKind.EXPENSE, "Transport")
    private var pending: PendingTransaction? = null
    private var posts = 0
    private var command: TransactionCreateCommand? = null

    private fun render(dark: Boolean = false, narrow: Boolean = false) {
        val backend = object : RecordingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun accounts(owner: OwnerId) = RepositoryResult.Success(listOf(daily))
            override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
                posts++; command = pending.command
                return RepositoryResult.Success(Unit)
            }
        }
        val store = object : PendingTransactionStore {
            override suspend fun read(owner: OwnerId) = pending
            override suspend fun save(pending: PendingTransaction) { this@ExpenseFormFidelityTest.pending = pending }
            override suspend fun clear(owner: OwnerId) { pending = null }
        }
        val flow = RecordingSession(backend, store, { RepositoryResult.Success(listOf(category)) },
            { IdempotencyKey("expense-preview-key") }, { "2026-11-18" })
        runBlocking { flow.begin() }
        flow.update(flow.state.value.form.copy(amount = "200.00", occurredOn = "2026-10-21", description = "Transport"))
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("expense-preview")) {
                    CuenoraTheme(darkTheme = dark) {
                        Surface(Modifier.fillMaxSize()) {
                            RecordingScreen(flow, { action -> scope.launch { action() } }, flow::cancel)
                        }
                    }
                }
            }
        }
    }

    @Test fun expenseUsesPenFieldOrderAndExplicitReviewWithoutPosting() {
        render()
        compose.onNodeWithText("Record expense").assertExists()
        compose.onNodeWithText("Transactions").assertExists()
        val tags = listOf("record-amount", "expense-currency", "record-date", "record-description", "expense-category", "expense-account")
        val bounds = tags.map { compose.onNodeWithTag(it).getUnclippedBoundsInRoot() }
        bounds.zipWithNext().forEach { (before, after) -> assertTrue(before.bottom <= after.top) }
        capture("expense-light-top")
        compose.onNodeWithText("Included in spending reports").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("No account balance will change.").assertExists()
        compose.onNodeWithText("Review expense").performScrollTo().assertIsDisplayed()
        capture("expense-light-actions")
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-review").performClick()
        compose.onNodeWithText("Review your record").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        compose.onNodeWithText("Record saved").assertExists()
        val expense = command as TransactionCreateCommand.Expense
        assertEquals(Money(20000, Currency.SEK), expense.amount)
        assertEquals(BusinessDate(2026, 10, 21), expense.occurredOn)
        assertEquals("Transport", expense.description)
        assertNull(expense.accountId); assertNull(expense.categoryId)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun darkExpenseKeepsSelectorsAndDiscardBehavior() {
        render(dark = true)
        capture("expense-dark-top")
        compose.onNodeWithTag("expense-currency").performScrollTo().performClick()
        compose.onNodeWithText("EUR").performClick()
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("expense-category").performScrollTo().performClick()
        compose.onNode(hasText("Transport") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("record-cancel").performScrollTo().performClick()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("record-description").assertTextContains("Transport")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun narrowLargeTextKeepsTypeSwitchingAndActionsReachable() {
        render(dark = true, narrow = true)
        capture("expense-dark-large-top")
        for (tag in listOf("record-INCOME", "record-TRANSFER", "expense-currency", "expense-category", "expense-account", "record-review", "record-cancel")) {
            val node = compose.onNodeWithTag(tag).performScrollTo().assertIsDisplayed()
            val bounds = node.getUnclippedBoundsInRoot()
            val viewport = compose.onNodeWithTag("expense-preview").getUnclippedBoundsInRoot()
            assertTrue("$tag must fit the viewport", bounds.left >= viewport.left && bounds.right <= viewport.right)
        }
        capture("expense-dark-large-actions")
        compose.onNodeWithTag("record-INCOME").performScrollTo().performClick()
        compose.onNodeWithText("Review record").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("record-TRANSFER").performScrollTo().performClick()
        compose.onNodeWithText("Review record").performScrollTo().assertIsDisplayed()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun expenseSelectorsPreserveAccountCurrencyValidationAndCategoryOnSave() {
        render()
        compose.onNodeWithTag("expense-category").performScrollTo().performClick()
        compose.onNode(hasText("Transport") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("expense-account").performScrollTo().performClick()
        compose.onNodeWithText("Daily").performClick()
        compose.onNodeWithTag("expense-currency").performScrollTo().performClick()
        compose.onNodeWithText("EUR").performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Choose an eligible same-currency account", substring = true).assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("expense-currency").performScrollTo().performClick()
        compose.onNodeWithText("SEK").performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Account: Daily").assertExists()
        compose.onNodeWithText("Category: Transport").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        val expense = command as TransactionCreateCommand.Expense
        assertEquals(daily.id, expense.accountId); assertEquals(category.id, expense.categoryId)
        assertEquals(Money(20000, Currency.SEK), expense.amount)
        assertEquals(1, posts); assertNull(pending)
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("expense-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
