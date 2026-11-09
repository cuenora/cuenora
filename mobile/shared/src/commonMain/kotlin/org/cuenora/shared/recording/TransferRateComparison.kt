package org.cuenora.shared.recording

import kotlinx.serialization.json.JsonPrimitive
import org.cuenora.shared.domain.Money
import org.cuenora.shared.domain.TransactionKind
import org.cuenora.shared.repository.RateComparison
import org.cuenora.shared.repository.readMoney
import org.cuenora.shared.validation.TransactionForm
import org.cuenora.shared.validation.validateTransactionForm

/** Exact quote comparison for reviewed actual amounts; never changes either input. */
internal fun transferRateComparison(form: TransactionForm): RateComparison? {
    if (form.kind != TransactionKind.TRANSFER || form.quotedRate == null) return null
    require(validateTransactionForm(form).isLocallyValid)
    val sent = readMoney(JsonPrimitive(form.amount), form.currency)
    val received = readMoney(JsonPrimitive(form.receivedAmount!!), form.receivedCurrency!!)
    val quote = form.quotedRate
    val scale = quote.substringAfter('.', "").length
    val digits = quote.replace(".", "").trimStart('0').ifEmpty { "0" }
    // Decimal digit multiplication avoids binary floats and intermediate Long overflow.
    // A source amount has at most 19 digits; each accumulator remains well below Int.MAX_VALUE.
    val left = sent.minorUnits.toString()
    val product = IntArray(left.length + digits.length)
    for (i in left.indices) for (j in digits.indices)
        product[i + j + 1] += (left[i] - '0') * (digits[j] - '0')
    for (i in product.lastIndex downTo 1) {
        product[i - 1] += product[i] / 10
        product[i] %= 10
    }
    val raw = product.joinToString("").trimStart('0').ifEmpty { "0" }.padStart(scale + 1, '0')
    val whole = if (scale == 0) raw else raw.dropLast(scale)
    val fraction = if (scale == 0) "" else raw.takeLast(scale)
    val minor = whole.toLongOrNull() ?: throw IllegalArgumentException("Indicative amount exceeds recorded range")
    val roundUp = fraction.isNotEmpty() && (fraction[0] > '5' || fraction[0] == '5' &&
        (fraction.drop(1).any { it != '0' } || minor % 2L == 1L))
    require(!roundUp || minor < Long.MAX_VALUE) { "Indicative amount exceeds recorded range" }
    val indicative = minor + if (roundUp) 1 else 0
    val difference = received.minorUnits - indicative
    return RateComparison(Money(indicative, received.currency), Money(difference, received.currency), difference != 0L)
}
