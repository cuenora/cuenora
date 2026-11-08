package org.cuenora.android.ui

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import org.cuenora.shared.domain.*
import org.cuenora.shared.history.HistoryReader
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*

/** Financial drafts and records deliberately live in memory, never saved instance state. */
data class HistoryFilterDraft(
    val from: String = "",
    val to: String = "",
    val account: TransactionAccountFilter? = null,
    val category: CategoryId? = null,
)
data class TransactionBrowserState(
    val items: List<TransactionRecord> = emptyList(),
    val nextCursor: String? = null,
    val loaded: Boolean = false,
    val busy: Boolean = false,
    val failure: ApiFailure? = null,
    val applied: HistoryFilterDraft = HistoryFilterDraft(),
    val draft: HistoryFilterDraft = HistoryFilterDraft(),
    val filtersOpen: Boolean = false,
    val filterError: String? = null,
    val accounts: List<AccountWithBalance> = emptyList(),
    val accountCursor: String? = null,
    val categories: List<CatalogCategory> = emptyList(),
    val choicesLoaded: Boolean = false,
    val choicesFailure: Boolean = false,
)

class TransactionBrowser(private val reader: HistoryReader) {
    private val mutable = MutableStateFlow(TransactionBrowserState())
    val state = mutable.asStateFlow()
    private val mutex = Mutex()
    private var retired = false
    private val cursors = mutableSetOf<String>()
    private val accountCursors = mutableSetOf<String>()

    suspend fun loadIfNeeded() { if (!state.value.loaded && state.value.failure == null) refresh() }
    suspend fun refresh() = load(false)
    suspend fun loadMore() = load(true)
    fun clear() {
        retired = true; cursors.clear(); accountCursors.clear()
        mutable.value = TransactionBrowserState()
    }
    fun openFilters() {
        if (!retired && !state.value.busy) mutable.value = state.value.copy(
            filtersOpen = true, draft = state.value.applied, filterError = null)
    }
    fun updateDraft(value: HistoryFilterDraft) {
        if (!retired && state.value.filtersOpen && !state.value.busy)
            mutable.value = state.value.copy(draft = value, filterError = null)
    }
    fun resetDraft() = updateDraft(HistoryFilterDraft())
    fun cancelFilters() {
        if (!retired && !state.value.busy) mutable.value = state.value.copy(filtersOpen = false,
            draft = state.value.applied, filterError = null)
    }
    suspend fun applyFilters() {
        if (retired || state.value.busy || !state.value.filtersOpen) return
        val draft = state.value.draft
        try { query(draft) } catch (_: IllegalArgumentException) {
            mutable.value = state.value.copy(filterError = "Use valid YYYY-MM-DD dates; the end cannot precede the start.")
            return
        }
        mutable.value = state.value.copy(applied = draft, filtersOpen = false, filterError = null)
        refresh()
    }

    /** Choices are paginated too: archived accounts stay available for historical reads. */
    suspend fun loadChoices(more: Boolean = false) {
        if (retired || !mutex.tryLock()) return
        try {
            val s = state.value
            val cursor = if (more) s.accountCursor ?: return else null
            if (!more) accountCursors.clear()
            mutable.value = s.copy(busy = true, choicesFailure = false)
            val accounts = reader.accounts(AccountListQuery(limit = 20, cursor = cursor))
            if (retired) return
            if (accounts is RepositoryResult.Failure) { choiceFailure(accounts.error); return }
            val page = (accounts as RepositoryResult.Success).value
            if (page.nextCursor != null && (page.nextCursor == cursor || page.nextCursor in accountCursors)) {
                choiceFailure(ApiFailure.InvalidResponse(200, null)); return
            }
            val categories = reader.categories()
            if (retired) return
            if (categories is RepositoryResult.Failure) { choiceFailure(categories.error); return }
            if (cursor != null) accountCursors += cursor
            mutable.value = state.value.copy(busy = false, choicesLoaded = true, choicesFailure = false,
                accounts = ((if (more) s.accounts else emptyList()) + page.items).distinctBy { it.account.id },
                accountCursor = page.nextCursor, categories = (categories as RepositoryResult.Success).value)
        } catch (cancelled: CancellationException) {
            if (!retired) mutable.value = state.value.copy(busy = false, choicesFailure = true)
            throw cancelled
        } finally { mutex.unlock() }
    }
    private fun choiceFailure(error: ApiFailure) {
        if (denied(error)) mutable.value = TransactionBrowserState(failure = error)
        else mutable.value = state.value.copy(busy = false, choicesFailure = true)
    }
    private suspend fun load(append: Boolean) {
        if (retired || !mutex.tryLock()) return
        try {
            val s = state.value
            val cursor = if (append) s.nextCursor ?: return else null
            if (!append) cursors.clear()
            mutable.value = s.copy(busy = true, failure = null,
                items = if (append) s.items else emptyList(), loaded = append && s.loaded,
                nextCursor = if (append) s.nextCursor else null)
            val result = reader.list(query(s.applied).copy(cursor = cursor))
            if (retired) return
            when (result) {
                is RepositoryResult.Failure -> fail(result.error, append)
                is RepositoryResult.Success -> {
                    val page = result.value
                    if (page.nextCursor != null && (page.nextCursor == cursor || page.nextCursor in cursors)) {
                        fail(ApiFailure.InvalidResponse(200, null), append)
                    } else {
                        if (cursor != null) cursors += cursor
                        mutable.value = state.value.copy(items = ((if (append) s.items else emptyList()) + page.items)
                            .distinctBy { it.transaction.id }, nextCursor = page.nextCursor,
                            busy = false, loaded = true, failure = null)
                    }
                }
            }
        } catch (cancelled: CancellationException) {
            if (!retired) mutable.value = state.value.copy(busy = false, failure = ApiFailure.Network)
            throw cancelled
        } finally { mutex.unlock() }
    }
    private fun fail(error: ApiFailure, append: Boolean) {
        mutable.value = if (denied(error)) TransactionBrowserState(failure = error)
        else state.value.copy(busy = false, failure = error, loaded = append && state.value.loaded)
    }
    private fun denied(error: ApiFailure) = (error as? ApiFailure.Http)?.status in setOf(401, 403)
    private fun query(filters: HistoryFilterDraft): TransactionQuery {
        fun date(value: String): BusinessDate? {
            if (value.isEmpty()) return null
            require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(value))
            return BusinessDate(value.substring(0, 4).toInt(), value.substring(5, 7).toInt(), value.substring(8, 10).toInt())
        }
        val from = date(filters.from); val to = date(filters.to)
        require(from == null || to == null || from <= to)
        return TransactionQuery(limit = 20, dateFrom = from, dateTo = to, account = filters.account, categoryId = filters.category)
    }
}
