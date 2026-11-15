package org.cuenora.android

import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import kotlinx.coroutines.launch
import org.cuenora.android.ui.AccountArchiveActions
import org.cuenora.android.ui.CuenoraTheme
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class AccountArchiveActionsTest {
    @get:Rule val compose = createComposeRule()

    private val account = Account(AccountId(0, 4), OwnerId(0, 7), AccountName("Daily"),
        AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)

    private fun render(
        minorUnits: Long = 0,
        update: suspend (Account, AccountStatus) -> RepositoryResult<Account>,
        confirmed: (Account) -> Unit = {},
        refreshed: suspend () -> Unit = {},
    ) {
        compose.setContent {
            val scope = rememberCoroutineScope()
            CuenoraTheme {
                AccountArchiveActions(
                    account = account,
                    balance = Money(minorUnits, Currency.SEK),
                    updateStatus = update,
                    submit = { action -> scope.launch { action() } },
                    onConfirmed = confirmed,
                    refreshAfterUncertainResult = refreshed,
                )
            }
        }
    }

    @Test fun nonZeroBalanceExplainsWhyArchiveIsUnavailable() {
        render(minorUnits = 1, update = { _, _ -> error("A nonzero balance must not submit") })
        compose.onNodeWithText("Archive account").assertIsNotEnabled()
        compose.onNodeWithText("Bring the posted balance to exactly zero before archiving this account.").assertExists()
    }

    @Test fun confirmationCanBeDismissedWithoutMutation() {
        var calls = 0
        render(update = { _, _ -> calls++; RepositoryResult.Success(account) })
        compose.onNodeWithText("Archive account").performClick()
        compose.onNodeWithText("The account and its history will be kept. You can reactivate it later.").assertExists()
        compose.onNodeWithTag("cancel-account-status").performClick()
        compose.waitForIdle()
        assertEquals(0, calls)
    }

    @Test fun rejectionDoesNotClaimSuccessAndConfirmedArchiveCanBeReactivated() {
        var first = true
        val requests = mutableListOf<AccountStatus>()
        var confirmed = 0
        var refreshes = 0
        render(
            update = { _, status ->
                requests += status
                if (first) {
                    first = false
                    RepositoryResult.Failure(ApiFailure.Http(409, null, null, null))
                } else RepositoryResult.Success(account.copy(status = status))
            },
            confirmed = { confirmed++ },
            refreshed = { refreshes++ },
        )
        compose.onNodeWithText("Archive account").performClick()
        compose.onNodeWithTag("confirm-account-status").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("account-status-message").assertTextContains("could not be archived", substring = true)
        assertEquals(0, confirmed)
        assertEquals(0, refreshes)

        compose.onNodeWithText("Archive account").performClick()
        compose.onNodeWithTag("confirm-account-status").performClick()
        compose.waitForIdle()
        compose.onNodeWithText("Account archived.").assertExists()
        compose.onNodeWithText("Reactivate account").assertIsDisplayed()
        assertEquals(listOf(AccountStatus.ARCHIVED, AccountStatus.ARCHIVED), requests)
        assertEquals(1, confirmed)
    }

    @Test fun uncertainResultRefreshesAuthoritativeStateWithoutClaimingSuccess() {
        var refreshes = 0
        var confirmed = false
        render(
            update = { _, _ -> RepositoryResult.Failure(ApiFailure.Timeout) },
            confirmed = { confirmed = true },
            refreshed = { refreshes++ },
        )
        compose.onNodeWithText("Archive account").performClick()
        compose.onNodeWithTag("confirm-account-status").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("account-status-message").assertTextContains("result is unknown", substring = true)
        assertEquals(1, refreshes)
        assertFalse(confirmed)
        compose.onNodeWithText("Archive account").assertIsNotEnabled()
    }
}
