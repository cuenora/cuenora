package org.cuenora.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import org.cuenora.shared.domain.Account
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.Money
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult

@Composable
fun AccountArchiveActions(
    account: Account,
    balance: Money,
    updateStatus: (suspend (Account, AccountStatus) -> RepositoryResult<Account>)?,
    submit: (suspend () -> Unit) -> Unit,
    onConfirmed: suspend (Account) -> Unit = {},
    refreshAfterUncertainResult: suspend () -> Unit = {},
) {
    var displayedStatus by remember(account.id, account.status) { mutableStateOf(account.status) }
    var confirming by remember(account.id) { mutableStateOf(false) }
    var busy by remember(account.id) { mutableStateOf(false) }
    var requiresRefresh by remember(account.id) { mutableStateOf(false) }
    var message by remember(account.id) { mutableStateOf<String?>(null) }

    val target = if (displayedStatus == AccountStatus.ACTIVE) AccountStatus.ARCHIVED else AccountStatus.ACTIVE
    val isArchiving = target == AccountStatus.ARCHIVED
    val nonZeroBalance = balance.minorUnits != 0L
    val unavailableReason = when {
        requiresRefresh -> "Refresh accounts to confirm the current status before another change."
        updateStatus == null -> "Account status changes are unavailable in this view."
        isArchiving && nonZeroBalance -> "Bring the posted balance to exactly zero before archiving this account."
        else -> null
    }

    CuenoraAction(
        label = if (isArchiving) "Archive account" else "Reactivate account",
        onClick = { confirming = true; message = null },
        modifier = Modifier.testTag("account-status-action"),
        unavailableReason = unavailableReason,
        busy = busy,
    )
    message?.let { Text(it, modifier = Modifier.testTag("account-status-message")) }

    if (requiresRefresh) TextButton(onClick = { submit { refreshAfterUncertainResult() } }, enabled = !busy) { Text("Refresh accounts") }

    if (confirming) {
        AlertDialog(
            onDismissRequest = { if (!busy) confirming = false },
            title = { Text(if (isArchiving) "Archive account?" else "Reactivate account?") },
            text = {
                Text(if (isArchiving)
                    "The account and its history will be kept. You can reactivate it later."
                else "This account will become active again. Its history will be kept.")
            },
            confirmButton = {
                TextButton(
                    onClick = click@{
                        if (busy || requiresRefresh) return@click
                        busy = true
                        val requestedStatus = target
                        confirming = false
                        submit {
                            try {
                                when (val result = updateStatus?.invoke(account, requestedStatus)) {
                                    is RepositoryResult.Success -> {
                                        val updated = result.value
                                        if (updated.id == account.id && updated.ownerId == account.ownerId &&
                                            updated.status == requestedStatus) {
                                            displayedStatus = updated.status
                                            message = if (requestedStatus == AccountStatus.ARCHIVED)
                                                "Account archived." else "Account reactivated."
                                            onConfirmed(updated)
                                        } else {
                                            requiresRefresh = true
                                            message = "The account status could not be confirmed. Refresh accounts before trying again."
                                            refreshAfterUncertainResult()
                                        }
                                    }
                                    is RepositoryResult.Failure -> {
                                        val unknown = result.error == ApiFailure.Network || result.error == ApiFailure.Timeout ||
                                            result.error is ApiFailure.InvalidResponse ||
                                            (result.error as? ApiFailure.Http)?.status?.let { it in 500..599 } == true
                                        message = if (unknown) {
                                            "The result is unknown. Refresh accounts before trying again."
                                        } else if (isArchiving) {
                                            "The account could not be archived. Check that its posted balance is exactly zero, then refresh accounts."
                                        } else {
                                            "The account could not be reactivated. Refresh accounts and try again."
                                        }
                                        if (unknown) { requiresRefresh = true; refreshAfterUncertainResult() }
                                    }
                                    null -> message = "Account status changes are unavailable in this view."
                                }
                            } catch (cancelled: kotlinx.coroutines.CancellationException) {
                                requiresRefresh = true
                                message = "The result is unknown. Refresh accounts before trying again."
                                throw cancelled
                            } finally {
                                busy = false
                            }
                        }
                    },
                    enabled = !busy,
                    modifier = Modifier.testTag("confirm-account-status"),
                ) { Text(if (isArchiving) "Archive" else "Reactivate") }
            },
            dismissButton = {
                TextButton(onClick = { confirming = false }, enabled = !busy,
                    modifier = Modifier.testTag("cancel-account-status")) { Text("Cancel") }
            },
        )
    }
}
