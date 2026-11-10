package org.cuenora.shared.planning

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import kotlin.test.*

class PlanningSessionTest {
    private fun report(date: BusinessDate) = AccountPlanning("2026-11-10T10:00:00Z", date, true, emptyList(), listOf(
        UnassignedPlanning(Currency.SEK,"0.00","0.00",true), UnassignedPlanning(Currency.EUR,"0.00","0.00",true), UnassignedPlanning(Currency.USD,"0.00","0.00",true)))

    @Test fun dateCutoffIsSentExactlyAndRetainedOnLoadIfNeeded() = runTest {
        val seen = mutableListOf<BusinessDate?>()
        val reader = object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> {
                seen += through
                return RepositoryResult.Success(report(through ?: BusinessDate(2026, 11, 30)))
            }
        }
        val session = PlanningSession(reader)
        session.applyThrough(BusinessDate(2027, 2, 28))
        session.loadIfNeeded()
        assertEquals(listOf<BusinessDate?>(BusinessDate(2027, 2, 28)), seen)
        assertEquals(BusinessDate(2027, 2, 28), session.state.value.selectedThrough)
        session.applyThrough(null)
        assertEquals(listOf<BusinessDate?>(BusinessDate(2027, 2, 28), null), seen)
        assertNull(session.state.value.selectedThrough)
    }

    @Test fun clearingWhileReadIsSuspendedCannotRepublish() = runTest {
        val started = CompletableDeferred<Unit>()
        val finish = CompletableDeferred<RepositoryResult<AccountPlanning>>()
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> {
                started.complete(Unit)
                return finish.await()
            }
        })
        val loading = async { session.loadIfNeeded() }
        started.await()
        session.clear()
        finish.complete(RepositoryResult.Success(report(BusinessDate(2026, 11, 30))))
        loading.await()
        assertNull(session.state.value.planning)
        assertFalse(session.state.value.loading)
        assertFalse(session.state.value.loaded)
    }

    @Test fun failedReadNeverPresentsFabricatedEmptyPlan() = runTest {
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?) = RepositoryResult.Failure(ApiFailure.Network)
        })
        session.loadIfNeeded()
        assertNull(session.state.value.planning)
        assertEquals(PlanningLoadFailure.Unavailable, session.state.value.failure)
        assertTrue(session.state.value.loaded.not())
    }

    @Test fun loadingDoesNotRetainPreviousSnapshotOrAcceptAnotherDate() = runTest {
        var calls = 0
        val started = CompletableDeferred<Unit>()
        val finish = CompletableDeferred<RepositoryResult<AccountPlanning>>()
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> {
                calls++
                if (calls == 1) return RepositoryResult.Success(report(BusinessDate(2026, 11, 30)))
                started.complete(Unit)
                return finish.await()
            }
        })
        session.loadIfNeeded()
        val refresh = launch { session.refresh() }
        started.await()
        assertNull(session.state.value.planning)
        session.applyThrough(BusinessDate(2027, 2, 28))
        assertNull(session.state.value.selectedThrough)
        assertEquals(2, calls)
        finish.complete(RepositoryResult.Success(report(BusinessDate(2026, 11, 30))))
        refresh.join()
        assertEquals(2, calls)
    }

    @Test fun concurrentInitialLoadsCoalesceAndFailureDoesNotAutoRetry() = runTest {
        var calls = 0
        val started = CompletableDeferred<Unit>()
        val finish = CompletableDeferred<RepositoryResult<AccountPlanning>>()
        val session = PlanningSession(object : PlanningReader {
            override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> {
                calls++
                started.complete(Unit)
                return finish.await()
            }
        })
        val first = launch { session.loadIfNeeded() }
        started.await()
        val second = launch { session.loadIfNeeded() }
        yield()
        assertEquals(1, calls)
        finish.complete(RepositoryResult.Failure(ApiFailure.Network))
        first.join(); second.join()
        session.loadIfNeeded()
        assertEquals(1, calls)
    }
}
