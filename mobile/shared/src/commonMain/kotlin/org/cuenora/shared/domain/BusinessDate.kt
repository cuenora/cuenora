package org.cuenora.shared.domain

/** Gregorian business date; carries no clock, zone or audit timestamp. */
data class BusinessDate(val year: Int, val month: Int, val day: Int) : Comparable<BusinessDate> {
    init {
        require(year in 1..9999 && month in 1..12) { "Invalid business date" }
        val leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
        val days = when (month) {
            2 -> if (leap) 29 else 28
            4, 6, 9, 11 -> 30
            else -> 31
        }
        require(day in 1..days) { "Invalid business date" }
    }

    override fun compareTo(other: BusinessDate): Int = when {
        year != other.year -> year.compareTo(other.year)
        month != other.month -> month.compareTo(other.month)
        else -> day.compareTo(other.day)
    }
}
