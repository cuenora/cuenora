package org.cuenora.shared.domain

private fun isUnsignedInteger(value: String): Boolean = value == "0" ||
    (value.isNotEmpty() && value[0] in '1'..'9' && value.all { it in '0'..'9' })

/** Exact nonnegative rational minor units; no narrowing, rounding or arithmetic. */
data class ExactAccrual(val numerator: String, val denominator: String) {
    init {
        require(isUnsignedInteger(numerator) && isUnsignedInteger(denominator) && denominator != "0") {
            "Accrual requires nonnegative integer numerator and positive integer denominator"
        }
    }
}

/** Arbitrary-precision projected minor units, never recorded Money or a receipt. */
data class ProjectedMinorUnits(val digits: String) {
    init {
        require(isUnsignedInteger(digits)) { "Projected minor units must be a nonnegative integer" }
    }
}

data class ForecastPeriod(
    val start: BusinessDate,
    val end: BusinessDate,
    val exactMinorUnits: ExactAccrual,
    /** Half-even rounded estimate at natural period close; null for an open tail. */
    val payoutMinorUnits: ProjectedMinorUnits?,
) {
    init {
        require(start <= end) { "Forecast period must not end before it starts" }
    }
}

/** Domain output only; snapshot assumptions and rate selection belong to use cases. */
class SavingsForecast(val currency: Currency, periods: List<ForecastPeriod>) {
    private val snapshot = periods.toList()
    val periods: List<ForecastPeriod> get() = snapshot.toList()

    override fun equals(other: Any?): Boolean =
        other is SavingsForecast && currency == other.currency && snapshot == other.snapshot
    override fun hashCode(): Int = 31 * currency.hashCode() + snapshot.hashCode()
}
