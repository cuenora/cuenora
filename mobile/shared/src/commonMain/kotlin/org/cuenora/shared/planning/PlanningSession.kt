package org.cuenora.shared.planning

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult

/** Pure presentation state for a read-only planning screen. */
class PlanningSession(private val reader: PlanningReader) {
    private val mutableState = MutableStateFlow(PlanningScreenState())
    val state: StateFlow<PlanningScreenState> = mutableState.asStateFlow()
    private val mutex = Mutex()
    private var retired = false
    private var generation = 0
    private var loadAttempted = false

    suspend fun loadIfNeeded() = load(state.value.selectedThrough, onlyIfNeeded = true)

    suspend fun refresh() = load(state.value.selectedThrough)

    /** Apply a civil date, or null to return to the server-owned default cutoff. */
    suspend fun applyThrough(date: BusinessDate?) = load(date, publishThroughOnSuccess = true)

    /** Clear data and permanently prevent late suspended results from being published. */
    fun clear() {
        retired = true
        generation++
        mutableState.value = PlanningScreenState()
    }

    private suspend fun load(
        through: BusinessDate?,
        onlyIfNeeded: Boolean = false,
        publishThroughOnSuccess: Boolean = false,
    ) {
        if (retired || state.value.loading) return
        mutex.withLock {
            if (retired || state.value.loading || onlyIfNeeded && (state.value.loaded || loadAttempted)) return@withLock
            loadAttempted = true
            val requestGeneration = generation
            mutableState.value = state.value.copy(planning = null, loaded = false, loading = true, failure = null)
            try {
                when (val result = reader.load(through)) {
                    is RepositoryResult.Success -> if (active(requestGeneration)) {
                        mutableState.value = state.value.copy(
                            planning = result.value,
                            selectedThrough = if (publishThroughOnSuccess) through else state.value.selectedThrough,
                            loaded = true, loading = false, failure = null,
                        )
                    }
                    is RepositoryResult.Failure -> if (active(requestGeneration)) {
                        mutableState.value = state.value.copy(planning = null, loaded = false, loading = false, failure = result.error.toPlanningFailure())
                    }
                }
            } catch (cancelled: CancellationException) {
                if (active(requestGeneration)) mutableState.value = state.value.copy(loading = false)
                throw cancelled
            } catch (_: Exception) {
                if (active(requestGeneration)) mutableState.value = state.value.copy(planning = null, loaded = false, loading = false, failure = PlanningLoadFailure.Unavailable)
            }
        }
    }

    private fun active(requestGeneration: Int) = !retired && generation == requestGeneration
}

private fun ApiFailure.toPlanningFailure(): PlanningLoadFailure = when (this) {
    is ApiFailure.Http -> if (status == 401 || status == 403) PlanningLoadFailure.AccessDenied else PlanningLoadFailure.Unavailable
    is ApiFailure.InvalidResponse -> PlanningLoadFailure.InvalidResponse
    ApiFailure.Network, ApiFailure.Timeout -> PlanningLoadFailure.Unavailable
}
