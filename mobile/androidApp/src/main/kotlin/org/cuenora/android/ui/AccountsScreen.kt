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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.cuenora.shared.domain.Account
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.planning.ExactDecimal
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.AccountWithBalance
import org.cuenora.shared.repository.RepositoryResult

@Composable
fun AccountsScreen(
    browser: AccountBrowser,
    recordActivity: ((Account) -> Unit)? = null,
    addAccount: (() -> Unit)? = null,
    addAccountEnabled: Boolean = true,
    savingsActivity: ((Account) -> Unit)? = null,
    updateAccountStatus: (suspend (Account, AccountStatus) -> RepositoryResult<Account>)? = null,
    refreshAccountViews: suspend () -> Unit = {},
    submit: (suspend () -> Unit) -> Unit,
) {
    val state by browser.state.collectAsState()
    val listState = rememberLazyListState()
    var showArchived by remember(browser) { mutableStateOf(false) }
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
                    Text("Record activity")
                }
            if (savingsActivity != null && selected.account.type == AccountType.SAVINGS)
                TextButton(onClick = { savingsActivity(selected.account) }, modifier = Modifier.testTag("savings-from-account")) {
                    Text("Savings terms and interest")
                }
            FinancialSection("Recorded balance", emphasis = true) {
                CuenoraAmount("Posted balance", selected.account.currency.name, accountAmount(selected.balance))
                AccountMetadata(selected)
            }
            AccountArchiveActions(
                account = selected.account,
                balance = selected.balance,
                updateStatus = updateAccountStatus,
                submit = submit,
                onConfirmed = { updated ->
                    browser.acceptStatusUpdate(updated)
                    refreshAccountViews()
                },
                refreshAfterUncertainResult = refreshAccountViews,
            )
            Text("Recorded movements only · planned payments and forecasts are separate.", style = MaterialTheme.typography.bodySmall)
        }
    } else {
        LazyColumn(state = listState, modifier = Modifier.fillMaxSize().testTag("account-list"),
            contentPadding = PaddingValues(horizontal = 20.dp, vertical = 16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    if (addAccount != null) CuenoraAction("Add account", addAccount, Modifier.testTag("add-account"),
                        unavailableReason = if (addAccountEnabled) null else "Wait for the current account operation to finish.")
                    Text("Recorded balances · grouped by currency", style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        FilterChip(selected = showArchived, onClick = { showArchived = !showArchived },
                            label = { Text("Show archived accounts") }, modifier = Modifier.testTag("show-archived-accounts"))
                        TextButton(onClick = { submit { browser.refresh() } }, enabled = !state.busy,
                            modifier = Modifier.cuenoraFocus()) { Text("Refresh accounts") }
                    }
                }
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
            Currency.entries.forEach { currency ->
                val group = state.items.filter { it.account.currency == currency && (showArchived || it.account.status == AccountStatus.ACTIVE) }
                if (group.isNotEmpty()) {
                    item(key = "currency-${currency.name}") {
                        FlowRow(Modifier.fillMaxWidth().padding(top = 12.dp).testTag("accounts-total-${currency.name}"),
                            horizontalArrangement = Arrangement.SpaceBetween, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text(if (state.nextCursor == null) "${currency.name} total" else "${currency.name} loaded balances",
                                style = MaterialTheme.typography.bodyMedium, modifier = Modifier.semantics { heading() })
                            Text("${currency.name} ${ExactDecimal.sum(group.map { accountAmount(it.balance) })}",
                                style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
                        }
                    }
                    items(group, key = { it.account.id.toString() }) { item ->
                        AccountOverviewRow(item, totalComplete = state.nextCursor == null) { browser.select(item.account.id) }
                    }
                }
            }
            if (state.loaded && state.items.isNotEmpty() && !showArchived && state.items.none { it.account.status == AccountStatus.ACTIVE }) item {
                Text("No active accounts in this page. Show archived accounts to view retained history.")
            }
            if (state.loaded && state.nextCursor != null && state.failure == null) item {
                TextButton(onClick = { submit { browser.loadMore() } }, enabled = !state.busy,
                    modifier = Modifier.cuenoraFocus()) { Text("Load more accounts") }
            }
            item {
                Text("Balances come from recorded account movements. Planned payments are shown separately.",
                    style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

/** PDaOP uses open rows and dividers; only the existing detail callback is invoked. */
@Composable
private fun AccountOverviewRow(item: AccountWithBalance, totalComplete: Boolean, open: () -> Unit) {
    Column(Modifier.fillMaxWidth().testTag("accounts-open-row-${item.account.id}").cuenoraFocus()
        .clickable(role = Role.Button, onClickLabel = "View account details", onClick = open)
        .padding(vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(item.account.name.value, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(24.dp))
        }
        Text("${accountType(item)} account${if (item.account.status == AccountStatus.ARCHIVED) " · Archived" else ""}",
            style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text("${item.account.currency.name} ${accountAmount(item.balance)}", fontFamily = UiFont,
            fontWeight = FontWeight.Medium, fontSize = 24.sp, lineHeight = 36.sp)
        if (item.account.type == AccountType.SAVINGS) {
            Text(if (totalComplete) "Included in the ${item.account.currency.name} total above."
                else "Included in the ${item.account.currency.name} loaded balances above.",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
    HorizontalDivider(Modifier.testTag("accounts-row-divider-${item.account.id}"))
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
