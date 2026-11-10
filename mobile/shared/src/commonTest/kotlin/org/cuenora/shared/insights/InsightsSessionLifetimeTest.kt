package org.cuenora.shared.insights

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.repository.RepositoryResult
import kotlin.test.*

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class InsightsSessionLifetimeTest {
    private val range = InsightsDateRange("2024-02-01", "2024-02-29")
    private fun report(selected: InsightsDateRange = range) = InsightsReport(selected, listOf(
        InsightsCurrencyReport(Currency.SEK, "1.00", "0.00", "1.00", emptyList(), emptyList()),
        InsightsCurrencyReport(Currency.EUR, "0.00", "0.00", "0.00", emptyList(), emptyList()),
        InsightsCurrencyReport(Currency.USD, "0.00", "0.00", "0.00", emptyList(), emptyList()),
    ))

    @Test fun clearDuringSuspendedLoadDoesNotRepublishAndPermanentlyRetiresSession() = runTest {
        val gate = CompletableDeferred<Unit>()
        var calls = 0
        val session = InsightsSession(InsightsReader {
            calls++
            gate.await()
            RepositoryResult.Success(report())
        })
        val loading = launch { session.loadIfNeeded() }
        runCurrent()
        session.clear()
        gate.complete(Unit)
        loading.join()
        assertNull(session.state.value.report)
        assertFalse(session.state.value.busy)
        session.loadIfNeeded()
        assertEquals(1, calls)
    }

    @Test fun cancellationAlwaysClearsBusyState() = runTest {
        val session = InsightsSession(InsightsReader { awaitCancellation() })
        val loading = launch { session.loadIfNeeded() }
        runCurrent()
        assertTrue(session.state.value.busy)
        loading.cancelAndJoin()
        assertFalse(session.state.value.busy)
    }

    @Test fun applyingNewDatesNeverShowsThePreviousPeriodReportWhileLoading() = runTest {
        val gate = CompletableDeferred<Unit>()
        var calls = 0
        val session = InsightsSession(InsightsReader { selected ->
            calls++
            if (calls == 1) RepositoryResult.Success(report(selected ?: range))
            else {
                gate.await()
                RepositoryResult.Success(report(selected!!))
            }
        })
        session.loadIfNeeded()
        assertNotNull(session.state.value.report)
        session.editDates()
        session.changeDateDraft("2024-03-01", "2024-03-31")
        val applying = launch { session.applyDateDraft() }
        runCurrent()
        assertNull(session.state.value.report)
        assertEquals("2024-03-01", session.state.value.range?.dateFrom)
        gate.complete(Unit)
        applying.join()
    }
}
