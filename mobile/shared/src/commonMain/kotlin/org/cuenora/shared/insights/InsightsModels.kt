package org.cuenora.shared.insights

import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.repository.businessDate
import org.cuenora.shared.repository.wireDate

/** Inclusive business-date range used by the existing report endpoints. */
data class InsightsDateRange(val dateFrom: String, val dateTo: String) {
    val calendarMonths: Int

    init {
        val from = businessDate(dateFrom)
        val to = businessDate(dateTo)
        require(from <= to) { "Start date must not be after end date" }
        calendarMonths = (to.year - from.year) * 12 + to.month - from.month + 1
        require(calendarMonths <= 120) { "Insights ranges may span at most 120 calendar months" }
    }

    companion object {
        fun currentMonth(profileToday: String): InsightsDateRange {
            val today = businessDate(profileToday)
            val first = BusinessDate(today.year, today.month, 1)
            val last = lastOfMonth(today.year, today.month)
            return InsightsDateRange(wireDate(first), wireDate(last))
        }
    }
}

data class InsightsCategory(
    val id: CategoryId?,
    val code: String?,
    val name: String?,
    val expenses: String,
    val percentage: String?,
)

data class InsightsMonth(
    val month: String,
    val dateFrom: String,
    val dateTo: String,
    val income: String,
    val expenses: String,
    val netFlow: String,
)

data class InsightsCurrencyReport(
    val currency: Currency,
    val income: String,
    val expenses: String,
    val netFlow: String,
    val months: List<InsightsMonth>,
    val categories: List<InsightsCategory>,
    /** Total from the independent expense-distribution endpoint, not guaranteed to match flows. */
    val distributionExpenses: String = expenses,
)

/** Combines three independent report reads; it is not an atomic server snapshot. */
data class InsightsReport(
    val range: InsightsDateRange,
    val currencies: List<InsightsCurrencyReport>,
)

internal fun expectedMonths(range: InsightsDateRange): List<Pair<String, Pair<String, String>>> {
    val from = businessDate(range.dateFrom)
    val to = businessDate(range.dateTo)
    val result = mutableListOf<Pair<String, Pair<String, String>>>()
    var year = from.year
    var month = from.month
    while (year < to.year || year == to.year && month <= to.month) {
        val start = BusinessDate(year, month, if (year == from.year && month == from.month) from.day else 1)
        val last = lastOfMonth(year, month)
        val end = if (last > to) to else last
        result += "${year.toString().padStart(4, '0')}-${month.toString().padStart(2, '0')}" to
            (wireDate(start) to wireDate(end))
        if (month == 12) { year++; month = 1 } else month++
    }
    return result
}

private fun lastOfMonth(year: Int, month: Int): BusinessDate {
    val leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    val day = when (month) {
        2 -> if (leap) 29 else 28
        4, 6, 9, 11 -> 30
        else -> 31
    }
    return BusinessDate(year, month, day)
}
