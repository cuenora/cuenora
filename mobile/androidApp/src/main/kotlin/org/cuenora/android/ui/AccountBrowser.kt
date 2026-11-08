package org.cuenora.android.ui

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import org.cuenora.shared.accounts.AccountReader
import org.cuenora.shared.domain.AccountId
import org.cuenora.shared.domain.Money
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*

/** Memory-only native presentation. Never put financial records in rememberSaveable or a Bundle. */
data class AccountBrowserState(
    val items: List<AccountWithBalance> = emptyList(),
    val nextCursor: String? = null,
    val selectedId: AccountId? = null,
    val loaded: Boolean = false,
    val busy: Boolean = false,
    val failure: ApiFailure? = null,
) {
    val selected: AccountWithBalance? get() = items.firstOrNull { it.account.id == selectedId }
}

class AccountBrowser(private val reader: AccountReader) {
    private val mutableState = MutableStateFlow(AccountBrowserState())
    val state = mutableState.asStateFlow()
    private val mutex = Mutex()
    private val consumedCursors = mutableSetOf<String>()
    private var closed = false

    suspend fun loadIfNeeded() {
        if (!state.value.loaded && state.value.failure == null) refresh()
    }
    suspend fun refresh() = load(append = false)
    suspend fun loadMore() = load(append = true)

    fun select(id: AccountId) {
        if (!closed && state.value.items.any { it.account.id == id })
            mutableState.value = state.value.copy(selectedId = id)
    }
    fun back() { mutableState.value = state.value.copy(selectedId = null) }

    /** Disposing the signed-in subtree permanently retires this browser, including pending reads. */
    fun clear() {
        closed = true
        consumedCursors.clear()
        mutableState.value = AccountBrowserState()
    }

    private suspend fun load(append: Boolean) {
        if (closed || !mutex.tryLock()) return
        try {
            val previous = state.value
            val cursor = if (append) previous.nextCursor ?: return else null
            if (!append) consumedCursors.clear()
            // A refreshed balance is unavailable until the new read succeeds, never a stale zero.
            mutableState.value = if (append) previous.copy(busy = true, failure = null)
                else AccountBrowserState(busy = true)
            val result = reader.list(AccountListQuery(limit = 20, cursor = cursor))
            if (closed) return
            when (result) {
                is RepositoryResult.Failure -> fail(result.error, append)
                is RepositoryResult.Success -> {
                    val page = result.value
                    val existing = if (append) state.value.items else emptyList()
                    val owner = existing.firstOrNull()?.account?.ownerId ?: page.items.firstOrNull()?.account?.ownerId
                    if (page.items.any { it.account.ownerId != owner }) {
                        fail(ApiFailure.Http(403, null, null, null), append)
                    } else if (page.nextCursor != null &&
                        (page.nextCursor == cursor || page.nextCursor in consumedCursors)) {
                        fail(ApiFailure.InvalidResponse(200, null), append)
                    } else {
                        if (cursor != null) consumedCursors += cursor
                        mutableState.value = state.value.copy(
                            items = (existing + page.items).distinctBy { it.account.id },
                            nextCursor = page.nextCursor, loaded = true, busy = false, failure = null,
                        )
                    }
                }
            }
        } catch (cancelled: CancellationException) {
            if (!closed) mutableState.value = state.value.copy(busy = false)
            throw cancelled
        } finally { mutex.unlock() }
    }

    private fun fail(error: ApiFailure, append: Boolean) {
        val denied = (error as? ApiFailure.Http)?.status in setOf(401, 403)
        mutableState.value = if (denied || !append) AccountBrowserState(failure = error)
            else state.value.copy(busy = false, failure = error)
    }
}

/** Explicit two-decimal recorded hundredths, including the full supported symmetric Long range. */
fun accountAmount(money: Money): String {
    val units = money.minorUnits
    val absolute = if (units < 0) -units else units
    return (if (units < 0) "-" else "") + (absolute / 100).toString() + "." +
        (absolute % 100).toString().padStart(2, '0')
}
