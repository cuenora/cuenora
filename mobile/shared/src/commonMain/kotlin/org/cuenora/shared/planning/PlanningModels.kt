package org.cuenora.shared.planning

import org.cuenora.shared.domain.AccountId
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.AccountType
import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.domain.Currency

data class PlanningAccount(
    val id: AccountId,
    val name: String,
    val type: AccountType,
    val status: AccountStatus,
    val currency: Currency,
    val currentBalance: String,
    val pendingTotal: String,
    val projectedTotal: String,
    val planningDifference: String?,
    val complete: Boolean,
)

data class UnassignedPlanning(val currency: Currency, val pendingTotal: String, val projectedTotal: String, val complete: Boolean)

data class AccountPlanning(
    val asOf: String,
    val through: BusinessDate,
    val complete: Boolean,
    val accounts: List<PlanningAccount>,
    val unassigned: List<UnassignedPlanning>,
)

enum class PlanningLoadFailure { AccessDenied, Unavailable, InvalidResponse }

data class PlanningScreenState(
    val planning: AccountPlanning? = null,
    val selectedThrough: BusinessDate? = null,
    val loaded: Boolean = false,
    val loading: Boolean = false,
    val failure: PlanningLoadFailure? = null,
)

/** Arbitrary-precision decimal arithmetic for exact two-place report amounts. */
object ExactDecimal {
    private val decimal = Regex("-?[0-9]+\\.[0-9]{2}")

    fun requireValid(value: String): String { require(decimal.matches(value)); return value }

    fun add(left: String, right: String): String {
        requireValid(left); requireValid(right)
        val (ls, lm) = minor(left)
        val (rs, rm) = minor(right)
        if (ls == rs) return format(ls, addMagnitude(lm, rm))
        val compare = compareMagnitude(lm, rm)
        return when {
            compare == 0 -> "0.00"
            compare > 0 -> format(ls, subtractMagnitude(lm, rm))
            else -> format(rs, subtractMagnitude(rm, lm))
        }
    }

    fun subtract(left: String, right: String): String {
        requireValid(right)
        return add(left, (if (right.startsWith('-')) right.drop(1) else "-$right"))
    }

    fun sum(values: Iterable<String>): String = values.fold("0.00", ::add)

    private fun minor(value: String): Pair<Boolean, String> {
        val negative = value.startsWith('-')
        val digits = value.removePrefix("-").filter { it != '.' }.trimStart('0').ifEmpty { "0" }
        return (negative && digits != "0") to digits
    }
    private fun format(negative: Boolean, magnitude: String): String {
        val digits = magnitude.trimStart('0').ifEmpty { "0" }.padStart(3, '0')
        return (if (negative && digits != "000") "-" else "") + digits.dropLast(2) + "." + digits.takeLast(2)
    }
    private fun compareMagnitude(a: String, b: String): Int = when {
        a.length != b.length -> a.length.compareTo(b.length)
        else -> a.compareTo(b)
    }
    private fun addMagnitude(a: String, b: String): String {
        val aa = a.reversed(); val bb = b.reversed(); val out = StringBuilder(); var carry = 0
        for (i in 0 until maxOf(aa.length, bb.length)) {
            val digit = (aa.getOrNull(i)?.digitToInt() ?: 0) + (bb.getOrNull(i)?.digitToInt() ?: 0) + carry
            out.append(digit % 10); carry = digit / 10
        }
        if (carry > 0) out.append(carry)
        return out.toString().reversed()
    }
    private fun subtractMagnitude(a: String, b: String): String {
        val aa = a.reversed(); val bb = b.reversed(); val out = StringBuilder(); var borrow = 0
        for (i in aa.indices) {
            var digit = aa[i].digitToInt() - borrow - (bb.getOrNull(i)?.digitToInt() ?: 0)
            borrow = if (digit < 0) { digit += 10; 1 } else 0
            out.append(digit)
        }
        return out.toString().reversed().trimStart('0').ifEmpty { "0" }
    }
}
