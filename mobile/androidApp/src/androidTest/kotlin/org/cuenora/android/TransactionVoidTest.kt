package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.*
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class TransactionVoidTest {
    @get:Rule val compose = createComposeRule()
    private val owner = OwnerId(0, 7)
    private val event = Transaction(TransactionId(0, 1), TransactionKind.EXPENSE, TransactionState.POSTED,
        Money(12345, Currency.SEK), BusinessDate(2026, 1, 2))

    @Test fun explicitConfirmationVoidsOnceAndCancellationDoesNotWrite() {
        val backend = Backend(); val store = Store()
        val session = TransactionVoidSession(backend, store) { IdempotencyKey("void-key-123") }
        val reader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery) = RepositoryResult.Success(Page(listOf(TransactionRecord(event)), null))
            override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page(emptyList<AccountWithBalance>(), null))
            override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
        }
        val browser = TransactionBrowser(reader)
        var refreshes = 0
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                TransactionsScreen(browser, { action -> scope.launch { action() } }, session, { refreshes++ }) {}
            }
        }

        val actionTag = "void-transaction-${event.id}"
        compose.onNodeWithTag(actionTag).assertDoesNotExist()
        compose.onNodeWithTag("transaction-${event.id}").performScrollTo().performClick()
        compose.onNodeWithTag("transaction-detail").assertExists()
        compose.onNodeWithTag(actionTag).performScrollTo().performClick()
        compose.onNodeWithText("Keep transaction").performClick()
        compose.runOnIdle { assertEquals(0, store.saves); assertEquals(0, backend.calls) }

        compose.onNodeWithTag(actionTag).performScrollTo().performClick()
        compose.onNodeWithTag("confirm-transaction-void").performClick()
        compose.waitUntil(5_000) { refreshes == 1 }
        compose.runOnIdle { assertEquals(1, store.saves); assertEquals(1, backend.calls); assertEquals(1, store.clears) }
        compose.onNodeWithText("Transaction voided").assertExists()
    }

    @Test fun editStartsOnExpenseDetailAndWritesOnlyAfterReview() {
        var stored: PendingExpenseEdit? = null
        var sent: PendingExpenseEdit? = null
        val edit = ExpenseEditSession({ owner }, {
            sent = it
            RepositoryResult.Success(TransactionAcknowledgement(event.id, TransactionState.POSTED))
        }, object : ExpenseEditJournal {
            override suspend fun read(owner: OwnerId) = stored
            override suspend fun save(pending: PendingExpenseEdit) { stored = pending }
            override suspend fun clear(owner: OwnerId) { stored = null }
        }) { IdempotencyKey("edit-key-123") }
        val reader = object : HistoryReader {
            override suspend fun list(query: TransactionQuery) = RepositoryResult.Success(Page(listOf(TransactionRecord(event)), null))
            override suspend fun accounts(query: AccountListQuery) = RepositoryResult.Success(Page(emptyList<AccountWithBalance>(), null))
            override suspend fun categories() = RepositoryResult.Success(emptyList<CatalogCategory>())
        }
        val browser = TransactionBrowser(reader)
        val void = TransactionVoidSession(Backend(), Store()) { IdempotencyKey("void-key-123") }
        var refreshes = 0
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                TransactionsScreen(browser, { action -> scope.launch { action() } }, void,
                    expenseEdit = edit, onEdited = { refreshes++ }) {}
            }
        }
        compose.onNodeWithTag("edit-transaction").assertDoesNotExist()
        compose.onNodeWithTag("transaction-${event.id}").performScrollTo().performClick()
        compose.onNodeWithTag("edit-transaction").performScrollTo().performClick()
        compose.onNodeWithTag("expense-edit-description").performScrollTo().performTextInput("Corrected")
        compose.onNodeWithTag("expense-edit-review").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(null, sent) }
        compose.onNodeWithTag("expense-edit-save").performScrollTo().performClick()
        compose.waitUntil(5_000) { refreshes == 1 }
        compose.runOnIdle {
            assertEquals(event.id, sent?.target)
            assertEquals(PatchField.Value("Corrected"), sent?.patch?.description)
            assertEquals(PatchField.Absent, sent?.patch?.accountId)
            assertEquals(PatchField.Absent, sent?.patch?.currency)
            assertEquals(null, stored)
        }
    }

    private class Store : PendingTransactionVoidStore {
        var saves = 0; var clears = 0
        override suspend fun read(owner: OwnerId): PendingTransactionVoid? = null
        override suspend fun save(pending: PendingTransactionVoid) { saves++ }
        override suspend fun clear(owner: OwnerId) { clears++ }
    }

    private inner class Backend : TransactionVoidBackend {
        var calls = 0
        override suspend fun profile() = RepositoryResult.Success(OnboardingProfile(owner, true, "Europe/Stockholm"))
        override suspend fun void(owner: OwnerId, transactionId: TransactionId, key: IdempotencyKey): RepositoryResult<TransactionAcknowledgement> {
            calls++
            return RepositoryResult.Success(TransactionAcknowledgement(transactionId, TransactionState.VOIDED))
        }
    }
}
