package org.cuenora.shared.domain

enum class Currency { SEK, EUR, USD }

/** Recorded hundredths, not a projection or a cross-currency total. */
data class Money(val minorUnits: Long, val currency: Currency) {
    init {
        require(minorUnits != Long.MIN_VALUE) { "Recorded amount exceeds the symmetric range" }
    }
}
