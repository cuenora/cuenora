package org.cuenora.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.ui.unit.dp
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import org.cuenora.shared.history.TransactionVoidSession
import org.cuenora.shared.history.TransactionVoidStage

@Composable
fun TransactionVoidHost(
    session: TransactionVoidSession,
    submit: (suspend () -> Unit) -> Unit,
    onVoided: () -> Unit,
) {
    LaunchedEffect(session) { submit { session.restorePending() } }
    val state by session.state.collectAsState()
    val transaction = state.transaction
    when (state.stage) {
        TransactionVoidStage.Idle -> Unit
        TransactionVoidStage.Confirming -> AlertDialog(
            onDismissRequest = session::cancel,
            title = { Text("Void this transaction?") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    transaction?.let {
                        Text(transactionKind(it.kind))
                        CuenoraAmount("Recorded amount", it.amount.currency.name, accountAmount(it.amount))
                        Text("Date: ${it.occurredOn.year}-${it.occurredOn.month.toString().padStart(2, '0')}-${it.occurredOn.day.toString().padStart(2, '0')}")
                        it.receivedAmount?.let { received -> CuenoraAmount("Received", received.currency.name, accountAmount(received)) }
                    }
                    Text("This reverses its posted effect while keeping the transaction in history. Transfers are voided as one operation.")
                }
            },
            confirmButton = {
                TextButton(onClick = { submit { if (session.confirm()) onVoided() } }, modifier = Modifier.testTag("confirm-transaction-void")) {
                    Text("Void transaction")
                }
            },
            dismissButton = { TextButton(onClick = session::cancel) { Text("Keep transaction") } },
        )
        TransactionVoidStage.Busy -> AlertDialog(
            onDismissRequest = {}, title = { Text("Voiding transaction") },
            text = { Column(verticalArrangement = Arrangement.spacedBy(12.dp)) { CircularProgressIndicator(); Text("Waiting for confirmation. Do not submit another operation.") } },
            confirmButton = {},
        )
        TransactionVoidStage.Uncertain -> AlertDialog(
            onDismissRequest = session::deferPending, title = { Text("Void not confirmed") },
            text = { Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(state.message ?: "The result is unclear. Retry the original operation.")
                if (state.recovered) state.targetId?.let { Text("Transaction reference: ${java.util.UUID(it.highBits, it.lowBits)}") }
            } },
            confirmButton = {
                TextButton(onClick = { session.retryOriginal(); submit { if (session.confirm()) onVoided() } },
                    modifier = Modifier.testTag("retry-original-transaction-void")) { Text("Retry original void") }
            },
            dismissButton = { TextButton(onClick = session::deferPending) { Text("Close for now") } },
        )
        TransactionVoidStage.Complete -> AlertDialog(
            onDismissRequest = session::dismiss, title = { Text("Transaction voided") },
            text = { Text(state.message ?: "History is retained.") },
            confirmButton = { TextButton(onClick = session::dismiss) { Text("Done") } },
        )
        TransactionVoidStage.Blocked -> AlertDialog(
            onDismissRequest = session::dismiss, title = { Text("Void unavailable") },
            text = { Text(state.message ?: "The operation could not be started.") },
            confirmButton = { TextButton(onClick = session::dismiss) { Text("Close") } },
        )
    }
}
