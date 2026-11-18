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

class IncomeFormFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val savings = daily.copy(id = AccountId(0, 10), name = AccountName("Savings"), type = AccountType.SAVINGS)
    private val euro = daily.copy(id = AccountId(0, 11), name = AccountName("Euro account"), currency = Currency.EUR)
    private val archived = daily.copy(id = AccountId(0, 12), name = AccountName("Archived"), status = AccountStatus.ARCHIVED)
    private val groceries = CatalogCategory(CategoryId(0, 5), "groceries", TransactionKind.EXPENSE, "Groceries")
    private val category = CatalogCategory(CategoryId(0, 4), "salary", TransactionKind.INCOME, "Salary")
    private var pending: PendingTransaction? = null
    private var posts = 0
    private var command: TransactionCreateCommand? = null

    private fun render(dark: Boolean = false, narrow: Boolean = false, account: Account? = null, unavailable: Boolean = false) {
        val backend = object : RecordingBackend {
            override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
            override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
            override suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>> =
                if (unavailable) RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.Network)
                else RepositoryResult.Success(listOf(daily, savings, euro, archived))
            override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
                posts++; command = pending.command
                return RepositoryResult.Success(Unit)
            }
        }
        val store = object : PendingTransactionStore {
            override suspend fun read(owner: OwnerId) = pending
            override suspend fun save(pending: PendingTransaction) { this@IncomeFormFidelityTest.pending = pending }
            override suspend fun clear(owner: OwnerId) { pending = null }
        }
        val flow = RecordingSession(backend, store, { if (unavailable) RepositoryResult.Failure(org.cuenora.shared.network.ApiFailure.Network) else RepositoryResult.Success(listOf(category, groceries)) },
            { IdempotencyKey("income-preview-key") }, { "2026-11-18" })
        runBlocking { flow.begin(account) }
        flow.update(flow.state.value.form.copy(kind = TransactionKind.INCOME, amount = "200.00", occurredOn = "2026-10-21", description = "Salary"))
        compose.setContent {
            val scope = rememberCoroutineScope()
            CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("income-preview")) {
                    CuenoraTheme(darkTheme = dark) {
                        Surface(Modifier.fillMaxSize()) {
                            RecordingScreen(flow, { action -> scope.launch { action() } }, flow::cancel)
                        }
                    }
                }
            }
        }
    }

    @Test fun incomeUsesPenFieldOrderAndExplicitReviewWithoutPosting() {
        render()
        compose.onNodeWithText("Record income").assertExists()
        compose.onNodeWithText("Transactions").assertExists()
        val tags = listOf("record-amount", "income-currency", "record-date", "record-description", "income-category", "income-account")
        val bounds = tags.map { compose.onNodeWithTag(it).getUnclippedBoundsInRoot() }
        bounds.zipWithNext().forEach { (before, after) -> assertTrue(before.bottom <= after.top) }
        capture("income-light-top")
        compose.onNodeWithText("Included in income reports").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("No account balance will change.").assertExists()
        compose.onNodeWithText("Review income").performScrollTo().assertIsDisplayed()
        capture("income-light-actions")
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-review").performClick()
        compose.onNodeWithText("Review your record").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        compose.onNodeWithText("Record saved").assertExists()
        val income = command as TransactionCreateCommand.Income
        assertEquals(Money(20000, Currency.SEK), income.amount)
        assertEquals(BusinessDate(2026, 10, 21), income.occurredOn)
        assertEquals("Salary", income.description)
        assertNull(income.accountId); assertNull(income.categoryId)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun darkIncomeKeepsSelectorsAndDiscardBehavior() {
        render(dark = true)
        capture("income-dark-top")
        compose.onNodeWithTag("income-currency").performScrollTo().performClick()
        compose.onNodeWithText("EUR").performClick()
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("income-category").performScrollTo().performClick()
        compose.onNode(hasText("Salary") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("record-cancel").performScrollTo().performClick()
        compose.onNodeWithText("Keep editing").performClick()
        compose.onNodeWithTag("record-description").assertTextContains("Salary")
        compose.onNodeWithContentDescription("Back").performClick()
        compose.onNodeWithText("Discard changes?").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun narrowLargeTextKeepsTypeSwitchingAndActionsReachable() {
        render(dark = true, narrow = true)
        capture("income-dark-large-top")
        for (tag in listOf("record-INCOME", "record-TRANSFER", "income-currency", "income-category", "income-account", "record-review", "record-cancel")) {
            val node = compose.onNodeWithTag(tag).performScrollTo().assertIsDisplayed()
            val bounds = node.getUnclippedBoundsInRoot()
            val viewport = compose.onNodeWithTag("income-preview").getUnclippedBoundsInRoot()
            assertTrue("$tag must fit the viewport", bounds.left >= viewport.left && bounds.right <= viewport.right)
        }
        capture("income-dark-large-actions")
        compose.onNodeWithTag("record-INCOME").performScrollTo().performClick()
        compose.onNodeWithText("Review income").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-amount").assertTextContains("200.00")
        compose.onNodeWithTag("record-TRANSFER").performScrollTo().performClick()
        compose.onNodeWithText("Review transfer").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("record-EXPENSE").performScrollTo().performClick()
        compose.onNodeWithText("Record expense").assertExists()
        assertEquals(0, posts); assertNull(pending)
    }

    @Test fun incomeSelectorsPreserveAccountCurrencyValidationAndCategoryOnSave() {
        render()
        compose.onNodeWithTag("income-account").performScrollTo().performClick()
        for (name in listOf("Savings", "Euro account", "Archived")) compose.onNodeWithText(name).assertDoesNotExist()
        compose.onNode(hasText("No account assigned") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("income-category").performScrollTo().performClick()
        compose.onNodeWithText("Groceries").assertDoesNotExist()
        compose.onNode(hasText("Salary") and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()
        compose.onNodeWithTag("income-account").performScrollTo().performClick()
        compose.onNodeWithText("Daily").performClick()
        compose.onNodeWithTag("income-currency").performScrollTo().performClick()
        compose.onNodeWithText("EUR").performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Choose an eligible same-currency account", substring = true).assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("income-currency").performScrollTo().performClick()
        compose.onNodeWithText("SEK").performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Account: Daily").assertExists()
        compose.onNodeWithText("Category: Salary").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        val income = command as TransactionCreateCommand.Income
        assertEquals(daily.id, income.accountId); assertEquals(category.id, income.categoryId)
        assertEquals(Money(20000, Currency.SEK), income.amount)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun preselectedSavingsRemainsVisibleAndRejectedUntilExplicitlyUnassigned() {
        render(account = savings)
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Choose an eligible same-currency account", substring = true).assertExists()
        compose.onNodeWithText("Review your record").assertDoesNotExist()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("income-account").performScrollTo().performClick()
        compose.onNode(hasText("Savings") and hasAnyAncestor(isPopup()), useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithText("No account assigned").performClick()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Account: No account assigned").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        assertNull((command as TransactionCreateCommand.Income).accountId)
        assertEquals(1, posts); assertNull(pending)
    }

    @Test fun unavailableOptionalChoicesStillAllowExplicitAccountlessIncome() {
        render(unavailable = true)
        compose.onNodeWithText("Accounts could not be loaded. You can record without an account, or cancel and retry later.").performScrollTo().assertExists()
        compose.onNodeWithText("Categories could not be loaded. You can record without a category, or cancel and retry later.").performScrollTo().assertExists()
        compose.onNodeWithTag("record-review").performScrollTo().performClick()
        compose.onNodeWithText("Review your record").assertExists()
        assertEquals(0, posts); assertNull(pending)
        compose.onNodeWithTag("record-save").performScrollTo().performClick()
        val income = command as TransactionCreateCommand.Income
        assertNull(income.accountId); assertNull(income.categoryId)
        assertEquals(Money(20000, Currency.SEK), income.amount)
        assertEquals(1, posts); assertNull(pending)
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use {
            compose.onNodeWithTag("income-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }
}
