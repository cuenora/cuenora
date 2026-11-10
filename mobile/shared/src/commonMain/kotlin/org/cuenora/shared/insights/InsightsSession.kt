package org.cuenora.shared.insights

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult

data class InsightsState(
    val range: InsightsDateRange? = null,
    val report: InsightsReport? = null,
    val selectedCurrency: Currency = Currency.SEK,
    val busy: Boolean = false,
    val loaded: Boolean = false,
    val error: ApiFailure? = null,
    val editingDates: Boolean = false,
    val draftFrom: String = "",
    val draftTo: String = "",
    val dateError: String? = null,
)

/** Session-scoped memory only: selected dates survive navigation, never credentials or reports across logout. */
class InsightsSession(private val reader: InsightsReader) {
    private val mutableState = MutableStateFlow(InsightsState())
    val state: StateFlow<InsightsState> = mutableState.asStateFlow()
    private var retired = false
    private var generation = 0

    suspend fun loadIfNeeded() {
        if (!retired && (!mutableState.value.loaded || mutableState.value.error != null) && !mutableState.value.busy)
            load(mutableState.value.range)
    }

    suspend fun refresh() = load(mutableState.value.range)

    fun editDates() {
        if (retired) return
        val current = mutableState.value
        val range = current.range
        mutableState.value = current.copy(
            editingDates = true,
            draftFrom = range?.dateFrom.orEmpty(),
            draftTo = range?.dateTo.orEmpty(),
            dateError = null,
        )
    }

    fun changeDateDraft(dateFrom: String = mutableState.value.draftFrom, dateTo: String = mutableState.value.draftTo) {
        if (retired) return
        mutableState.value = mutableState.value.copy(draftFrom = dateFrom, draftTo = dateTo, dateError = null)
    }

    suspend fun applyDateDraft(): Boolean {
        if (retired || mutableState.value.busy) return false
        val current = mutableState.value
        val range = runCatching { InsightsDateRange(current.draftFrom, current.draftTo) }.getOrElse {
            mutableState.value = current.copy(dateError = it.message ?: "Enter a valid inclusive date range.")
            return false
        }
        mutableState.value = current.copy(range = range, report = null, editingDates = false, dateError = null)
        load(range)
        return !mutableState.value.busy && mutableState.value.error == null
    }

    fun cancelDateDraft() {
        if (retired) return
        mutableState.value = mutableState.value.copy(
            editingDates = false,
            draftFrom = mutableState.value.range?.dateFrom.orEmpty(),
            draftTo = mutableState.value.range?.dateTo.orEmpty(),
            dateError = null,
        )
    }

    fun selectCurrency(currency: Currency) {
        if (retired) return
        if (mutableState.value.report?.currencies?.any { it.currency == currency } == true)
            mutableState.value = mutableState.value.copy(selectedCurrency = currency)
    }

    /** Re-reads the profile-zone default month; device-local time is never consulted. */
    suspend fun resetDateRange() = load(null)

    fun clear() {
        if (retired) return
        retired = true
        generation++
        (reader as? AuthenticatedInsightsReader)?.clear()
        mutableState.value = InsightsState()
    }

    private suspend fun load(range: InsightsDateRange?) {
        if (retired || mutableState.value.busy) return
        val startedAt = generation
        mutableState.value = mutableState.value.copy(
            range = range ?: mutableState.value.range,
            report = null,
            busy = true,
            error = null,
        )
        try {
            when (val result = reader.load(range)) {
                is RepositoryResult.Failure -> if (!retired && startedAt == generation) {
                    mutableState.value = mutableState.value.copy(
                        report = null,
                        busy = false,
                        loaded = true,
                        error = result.error,
                    )
                }
                is RepositoryResult.Success -> if (!retired && startedAt == generation) {
                    mutableState.value = mutableState.value.copy(
                        range = result.value.range,
                        report = result.value,
                        busy = false,
                        loaded = true,
                        error = null,
                        draftFrom = result.value.range.dateFrom,
                        draftTo = result.value.range.dateTo,
                    )
                }
            }
        } finally {
            if (!retired && startedAt == generation && mutableState.value.busy)
                mutableState.value = mutableState.value.copy(busy = false)
        }
    }
}
