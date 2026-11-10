package org.cuenora.shared.insights

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class InsightsDateRangeTest {
    @Test fun inclusiveRangesAcceptLeapDaysAndExactlyOneHundredTwentyCalendarMonths() {
        assertEquals("2024-02-29", InsightsDateRange("2024-02-01", "2024-02-29").dateTo)
        assertEquals(
            120,
            InsightsDateRange("2016-02-29", "2026-01-31").calendarMonths,
        )
    }

    @Test fun invalidReversedAndOverlongRangesAreRejected() {
        assertFailsWith<IllegalArgumentException> { InsightsDateRange("2025-02-29", "2025-03-01") }
        assertFailsWith<IllegalArgumentException> { InsightsDateRange("2025-04-02", "2025-04-01") }
        assertFailsWith<IllegalArgumentException> { InsightsDateRange("2016-01-01", "2026-01-01") }
    }

    @Test fun profileDateSelectsTheWholeCurrentCalendarMonth() {
        assertEquals(
            InsightsDateRange("2024-02-01", "2024-02-29"),
            InsightsDateRange.currentMonth("2024-02-29"),
        )
        assertEquals(
            InsightsDateRange("2026-11-01", "2026-11-30"),
            InsightsDateRange.currentMonth("2026-11-10"),
        )
    }
}
