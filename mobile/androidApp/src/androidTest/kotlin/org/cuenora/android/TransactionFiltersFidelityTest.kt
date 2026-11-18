package org.cuenora.android

import android.app.Activity
import android.graphics.Bitmap
import android.view.KeyEvent
import android.view.WindowManager
import android.widget.EditText
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click as clickView
import androidx.test.espresso.action.ViewActions.replaceText
import androidx.test.espresso.matcher.RootMatchers.isDialog
import androidx.test.espresso.Root
import org.hamcrest.Description
import org.hamcrest.TypeSafeMatcher
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.CompletableDeferred
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class TransactionFiltersFidelityTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val daily = Account(AccountId(0, 9), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val archived = daily.copy(id = AccountId(0, 10), name = AccountName("Archived savings"), currency = Currency.EUR,
        type = AccountType.SAVINGS, status = AccountStatus.ARCHIVED)
    private val food = CatalogCategory(CategoryId(0, 4), "food", TransactionKind.EXPENSE, "Food")
    private val queries = mutableListOf<TransactionQuery>()
    private var offline = false
    private var moreGate: CompletableDeferred<Unit>? = null
    private var activity: Activity? = null
    private lateinit var browser: TransactionBrowser

    private fun render(direct: Boolean = false, narrow: Boolean = false, dark: Boolean = false, startOpen: Boolean = true) {
        val reader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>> {
                queries += query
                return RepositoryResult.Success(Page(emptyList(), null))
            }
            override suspend fun accounts(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> {
                if (query.cursor != null) moreGate?.await()
                return if (offline) RepositoryResult.Failure(ApiFailure.Network)
                else RepositoryResult.Success(Page(listOf(AccountWithBalance(if (query.cursor == null) daily else archived,
                    Money(0, if (query.cursor == null) Currency.SEK else Currency.EUR))), if (query.cursor == null) "accounts-page-2" else null))
            }
            override suspend fun categories() = RepositoryResult.Success(listOf(food))
        }
        browser = TransactionBrowser(reader)
        runBlocking {
            browser.loadIfNeeded()
            if (startOpen) { browser.openFilters(); browser.loadChoices() }
        }
        if (direct) browser.updateDraft(HistoryFilterDraft(from = "2026-10-01", to = "2026-10-31"))
        compose.setContent {
            activity = LocalActivity.current
            val scope = rememberCoroutineScope()
            val state by browser.state.collectAsState()
            CuenoraTheme(darkTheme = dark) {
                if (direct) CompositionLocalProvider(LocalDensity provides Density(1f, if (narrow) 1.5f else 1f)) {
                    Box(Modifier.width(if (narrow) 320.dp else 390.dp).height(844.dp).testTag("filters-preview")) {
                        Surface(Modifier.fillMaxSize()) {
                            if (state.filtersOpen) TransactionFiltersScreen(browser, { action -> scope.launch { action() } })
                            else Text("Filters closed")
                        }
                    }
                } else Surface(Modifier.fillMaxSize()) {
                    TransactionsScreen(browser, { action -> scope.launch { action() } }, {})
                }
            }
        }
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()
    private fun choose(text: String) = compose.onNode(hasText(text) and hasAnyAncestor(isPopup()), useUnmergedTree = true).performClick()

    @Test fun filtersUsePenStructureAndOnlyApplyRefreshesHistory() {
        render()
        compose.onNodeWithText("Filter transactions").assertExists()
        compose.onNodeWithContentDescription("Back to transactions").assertExists()
        compose.onNodeWithTag("history-filter-currency").assertExists()
        compose.onNodeWithTag("history-filter-kind").assertExists()
        val tags = listOf("history-from", "history-to", "history-filter-account", "history-filter-accountless",
            "history-filter-kind", "history-filter-currency", "history-filter-category", "history-filter-apply", "history-filter-reset", "history-filter-cancel")
        tags.map { compose.onNodeWithTag(it).getUnclippedBoundsInRoot() }.zipWithNext().forEach { (a, b) -> assertTrue(a.bottom <= b.top) }
        click("history-filter-accountless")
        click("history-filter-kind"); choose("Expense")
        click("history-filter-currency"); choose("EUR")
        click("history-filter-category"); choose("Food · Expense")
        compose.runOnIdle { assertEquals(1, queries.size); assertEquals(HistoryFilterDraft(), browser.state.value.applied) }
        click("history-filter-apply")
        compose.onNodeWithText("No matching transactions").assertExists()
        compose.runOnIdle {
            assertEquals(2, queries.size); assertEquals(TransactionAccountFilter.Accountless, queries.last().account)
            assertEquals(TransactionKind.EXPENSE, queries.last().kind)
            assertEquals(Currency.EUR, queries.last().currency)
            assertEquals(food.id, queries.last().categoryId); assertNull(queries.last().cursor)
        }
    }

    @Test fun resetAndCancelKeepAppliedFiltersUntilExplicitApply() {
        render(); click("history-filter-accountless"); click("history-filter-apply")
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        click("history-filter-reset")
        compose.runOnIdle {
            assertEquals(HistoryFilterDraft(), browser.state.value.draft)
            assertEquals(TransactionAccountFilter.Accountless, browser.state.value.applied.account); assertEquals(2, queries.size)
        }
        click("history-filter-cancel")
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        compose.onNodeWithTag("history-filter-accountless").assertIsSelected()
        compose.runOnIdle { assertEquals(browser.state.value.applied, browser.state.value.draft); assertEquals(2, queries.size) }
        click("history-filter-reset"); click("history-filter-apply")
        compose.runOnIdle { assertEquals(3, queries.size); assertNull(queries.last().account); assertNull(queries.last().categoryId) }
    }

    @Test fun manualDateRangeValidationPreservesDraftUntilOneExplicitApply() {
        render()
        enterDate("history-from", "2026-10-31"); enterDate("history-to", "2026-10-01")
        click("history-filter-apply")
        compose.onNodeWithText("Use valid YYYY-MM-DD dates; the end cannot precede the start.").performScrollTo().assertExists()
        compose.runOnIdle { assertEquals(1, queries.size); assertEquals(HistoryFilterDraft(), browser.state.value.applied) }
        enterDate("history-from", "2026-10-01"); enterDate("history-to", "2026-10-31")
        compose.onNodeWithText("More account choices").performScrollTo().performClick()
        click("history-filter-account"); choose("Archived savings · EUR · Archived")
        click("history-filter-category"); choose("Food · Expense")
        click("history-filter-apply")
        compose.runOnIdle {
            assertEquals(2, queries.size); assertEquals(BusinessDate(2026, 10, 1), queries.last().dateFrom)
            assertEquals(BusinessDate(2026, 10, 31), queries.last().dateTo)
            assertEquals(TransactionAccountFilter.Linked(archived.id), queries.last().account)
            assertEquals(food.id, queries.last().categoryId); assertNull(queries.last().cursor)
        }
    }

    @Test fun metadataRetryPaginationAndExclusiveAccountChoicesKeepHistoryUnchanged() {
        offline = true; render()
        compose.onNodeWithText("Filter choices unavailable. Existing selections are unchanged.").performScrollTo().assertExists()
        offline = false
        compose.onNodeWithText("Retry filter choices").performScrollTo().performClick()
        compose.onNodeWithText("More account choices").performScrollTo().performClick()
        click("history-filter-account"); choose("Archived savings · EUR · Archived")
        compose.runOnIdle { assertEquals(TransactionAccountFilter.Linked(archived.id), browser.state.value.draft.account) }
        click("history-filter-accountless")
        compose.runOnIdle { assertEquals(TransactionAccountFilter.Accountless, browser.state.value.draft.account) }
        click("history-filter-account"); choose("All accounts")
        compose.runOnIdle { assertNull(browser.state.value.draft.account); assertEquals(1, queries.size) }
        click("history-filter-cancel")
        compose.runOnIdle { assertFalse(browser.state.value.filtersOpen); assertEquals(1, queries.size) }
    }

    @Test fun busyGuardsAndBackPreserveAppliedStateAndClosePopupFirst() {
        render()
        val gate = CompletableDeferred<Unit>(); moreGate = gate
        compose.onNodeWithText("More account choices").performScrollTo().performClick()
        compose.waitUntil { browser.state.value.busy }
        compose.onNodeWithText("Apply filters").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithTag("history-filter-reset").assertIsNotEnabled()
        compose.onNodeWithTag("history-filter-cancel").assertIsNotEnabled()
        compose.onNodeWithTag("history-filter-back").assertIsNotEnabled()
        back()
        compose.runOnIdle { assertTrue(browser.state.value.filtersOpen); assertEquals(1, queries.size); gate.complete(Unit) }
        compose.waitUntil { !browser.state.value.busy }
        back()
        compose.runOnIdle { assertFalse(browser.state.value.filtersOpen); assertEquals(1, queries.size) }
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        click("history-filter-accountless"); compose.onNodeWithTag("history-filter-back").performClick()
        compose.runOnIdle { assertEquals(HistoryFilterDraft(), browser.state.value.applied); assertEquals(1, queries.size) }
        compose.onNodeWithText("Filters").performScrollTo().performClick()
        click("history-filter-account"); assertPopupReady(); back()
        compose.onNode(isPopup()).assertDoesNotExist()
        compose.runOnIdle { assertTrue(browser.state.value.filtersOpen); assertEquals(1, queries.size) }
        back()
        compose.runOnIdle { assertFalse(browser.state.value.filtersOpen); assertEquals(1, queries.size) }
    }

    @Test fun fullScreenFilterDialogInheritsSecureActivityPolicy() {
        render(startOpen = false)
        compose.runOnIdle { activity!!.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE) }
        try {
            compose.onNodeWithText("Filters").performScrollTo().performClick()
            compose.onNodeWithTag("transaction-filters-screen").assertIsDisplayed()
            onView(isRoot()).inRoot(isDialog()).check { view, error ->
                if (error != null) throw error
                val flags = (view.rootView.layoutParams as WindowManager.LayoutParams).flags
                assertTrue(flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
            }
            click("history-filter-account")
            assertPopupReady()
            onView(isRoot()).inRoot(focusedPopup).check { view, error ->
                if (error != null) throw error
                val flags = (view.rootView.layoutParams as WindowManager.LayoutParams).flags
                assertTrue("Account popup must inherit secure policy", flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
            }
            back()
            compose.runOnIdle { assertTrue(browser.state.value.filtersOpen) }
            compose.onNodeWithTag("history-filter-back").performClick()
        } finally {
            compose.runOnIdle { activity!!.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE) }
        }
    }

    @Test fun lightPreviewKeepsBusinessDateMeaningAndStackedActions() {
        render(direct = true)
        capture("filters-light-top")
        compose.onNodeWithText("Dates refer to when the activity happened, not when a record was edited.").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("history-filter-cancel").performScrollTo().assertIsDisplayed()
        capture("filters-light-actions")
        compose.runOnIdle { assertEquals(1, queries.size) }
    }

    @Test fun darkNarrowLargeTextKeepsEveryControlWithinViewport() {
        render(direct = true, narrow = true, dark = true)
        capture("filters-dark-large-top")
        for (tag in listOf("history-from", "history-to", "history-filter-account", "history-filter-accountless", "history-filter-category",
            "history-filter-apply", "history-filter-reset", "history-filter-cancel")) {
            val bounds = compose.onNodeWithTag(tag).performScrollTo().assertIsDisplayed().getUnclippedBoundsInRoot()
            val viewport = compose.onNodeWithTag("filters-preview").getUnclippedBoundsInRoot()
            assertTrue("$tag must fit the viewport", bounds.left >= viewport.left && bounds.right <= viewport.right)
        }
        capture("filters-dark-large-actions")
        click("history-filter-cancel")
        compose.onNodeWithText("Filters closed").assertExists()
        compose.runOnIdle { assertEquals(1, queries.size); assertEquals(HistoryFilterDraft(), browser.state.value.applied) }
    }

    private fun enterDate(tag: String, value: String) {
        compose.onNodeWithTag(tag).performScrollTo()
        compose.onNode(hasContentDescription("Enter date manually") and hasAnyAncestor(hasTestTag(tag)), useUnmergedTree = true)
            .performSemanticsAction(SemanticsActions.OnClick) { it() }
        onView(isAssignableFrom(EditText::class.java)).inRoot(isDialog()).perform(replaceText(value))
        onView(withText(android.R.string.ok)).inRoot(isDialog()).perform(clickView())
        compose.onNodeWithTag(tag).assertTextContains(value)
    }

    private val focusedPopup = object : TypeSafeMatcher<Root>() {
        override fun describeTo(description: Description) { description.appendText("focused application sub-panel") }
        override fun matchesSafely(root: Root): Boolean = root.windowLayoutParams.isPresent &&
            root.windowLayoutParams.get().type == WindowManager.LayoutParams.TYPE_APPLICATION_SUB_PANEL && root.decorView.hasWindowFocus()
    }

    private fun assertPopupReady() {
        compose.onNode(isPopup()).assertIsDisplayed()
        onView(isRoot()).inRoot(focusedPopup).check { view, error ->
            if (error != null) throw error
            assertTrue(view.hasWindowFocus())
        }
    }

    private fun back() {
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        compose.waitForIdle()
    }

    private fun capture(name: String) {
        val file = File(InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "$name.png")
        file.outputStream().use { compose.onNodeWithTag("filters-preview").captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

}
