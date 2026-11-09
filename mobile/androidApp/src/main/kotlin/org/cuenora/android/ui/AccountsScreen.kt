package org.cuenora.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.AccountWithBalance

@Composable
fun AccountsScreen(browser: AccountBrowser, recordActivity: ((org.cuenora.shared.domain.Account) -> Unit)? = null, addAccount: (() -> Unit)? = null, addAccountEnabled: Boolean = true, submit: (suspend () -> Unit) -> Unit) {
    val state by browser.state.collectAsState()
    val listState = rememberLazyListState()
    LaunchedEffect(browser) { browser.loadIfNeeded() }
    val selected = state.selected
    BackHandler(enabled = selected != null) { browser.back() }
    if (selected != null) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)) {
            TextButton(onClick = browser::back, modifier = Modifier.cuenoraFocus()) { Text("Back to accounts") }
            AccountHeading("Account details")
            Text(selected.account.name.value, style = MaterialTheme.typography.headlineSmall)
            if (recordActivity != null && selected.account.status == AccountStatus.ACTIVE)
                TextButton(onClick = { recordActivity(selected.account) }, modifier = Modifier.testTag("record-from-account")) {
                    Text("Record income or expense")
                }
            AccountMetadata(selected)
            CuenoraAmount("Posted balance", selected.account.currency.name, accountAmount(selected.balance))
            Text("From the last successful account read. Planned payments and estimated interest are not included.")
            Text("To read current balances, return to the list and refresh accounts.")
        }
    } else {
        LazyColumn(state = listState, modifier = Modifier.fillMaxSize().testTag("account-list"),
            contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
            item {
                AccountHeading("Your accounts")
                Text("Posted balances, kept separate by currency.")
                if (addAccount != null) CuenoraAction("Add account", addAccount, Modifier.testTag("add-account"),
                    unavailableReason = if (addAccountEnabled) null else "Wait for the current account operation to finish.")
                TextButton(onClick = { submit { browser.refresh() } }, enabled = !state.busy,
                    modifier = Modifier.cuenoraFocus()) { Text("Refresh accounts") }
            }
            if (state.busy) item { CuenoraLoading(if (state.loaded) "Loading more accounts…" else "Loading accounts…") }
            state.failure?.let { failure ->
                item {
                    val more = state.loaded
                    CuenoraNotice(if (more) "More accounts could not be loaded" else "Accounts could not be loaded",
                        accountReadMessage(failure), NoticeTone.Error)
                    TextButton(onClick = { submit { if (more) browser.loadMore() else browser.refresh() } },
                        enabled = !state.busy, modifier = Modifier.cuenoraFocus()) {
                        Text(if (more) "Retry more accounts" else "Try again")
                    }
                }
            }
            if (state.loaded && state.items.isEmpty()) item {
                AccountHeading("No accounts yet")
                Text("Set up your first account from Home.")
                Text("An account is optional for income and expense records.")
            }
            items(state.items, key = { it.account.id.toString() }) { item ->
                Column(Modifier.fillMaxWidth().heightIn(min = 56.dp).cuenoraFocus()
                    .clickable(role = Role.Button, onClickLabel = "View account details") { browser.select(item.account.id) }
                    .padding(vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(item.account.name.value, style = MaterialTheme.typography.titleLarge)
                    Text("${accountType(item)} · ${accountStatus(item)}")
                    CuenoraAmount("Posted balance", item.account.currency.name, accountAmount(item.balance))
                }
                HorizontalDivider()
            }
            if (state.loaded && state.nextCursor != null && state.failure == null) item {
                TextButton(onClick = { submit { browser.loadMore() } }, enabled = !state.busy,
                    modifier = Modifier.cuenoraFocus()) { Text("Load more accounts") }
            }
        }
    }
}

@Composable
private fun AccountHeading(text: String) {
    Text(text, style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
}

@Composable
private fun AccountMetadata(item: AccountWithBalance) {
    Text("Type: ${accountType(item)}")
    Text("Currency: ${item.account.currency.name}")
    Text("Status: ${accountStatus(item)}")
}

private fun accountType(item: AccountWithBalance) = if (item.account.type == AccountType.NORMAL) "Normal" else "Savings"
private fun accountStatus(item: AccountWithBalance) = if (item.account.status == AccountStatus.ACTIVE) "Active" else "Archived"
private fun accountReadMessage(error: ApiFailure): String = when (error) {
    ApiFailure.Network, ApiFailure.Timeout -> "Current data is unavailable. Check your connection and try again. No financial records were changed."
    is ApiFailure.Http -> when (error.status) {
        401 -> "Your session is unavailable. Sign in again to read your accounts."
        403 -> "Account access is unavailable. Check email verification from Home and try again."
        429 -> "Too many requests. Wait before trying again."
        else -> "The service could not load accounts. Try again later."
    }
    is ApiFailure.InvalidResponse -> "The account response could not be read safely. Try refreshing accounts."
}
