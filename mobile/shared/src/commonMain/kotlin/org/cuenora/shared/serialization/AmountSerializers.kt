package org.cuenora.shared.serialization

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonPrimitive
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.Money
import org.cuenora.shared.domain.ProjectedMinorUnits

private val recordedDecimal = Regex("-?[0-9]+(?:\\.[0-9]{1,2})?")
private val reportDecimal = Regex("-?[0-9]+\\.[0-9]{2}")
private val projectedDecimal = Regex("[0-9]+\\.[0-9]{2}")

private fun invalidAmount(): Nothing = throw SerializationException("Invalid exact decimal amount")

private fun Decoder.amountString(): String {
    if (this !is JsonDecoder) return decodeString()
    // Checking the token kind also rejects numbers under lenient JSON settings.
    val primitive = decodeJsonElement() as? JsonPrimitive ?: invalidAmount()
    if (!primitive.isString) invalidAmount()
    return primitive.content
}

private fun minorDigits(unsigned: String): String {
    val whole = unsigned.substringBefore('.')
    val fraction = unsigned.substringAfter('.', "").padEnd(2, '0')
    return (whole + fraction).trimStart('0').ifEmpty { "0" }
}

private fun decimalDigits(minor: String): String {
    val padded = minor.padStart(3, '0')
    return padded.dropLast(2) + "." + padded.takeLast(2)
}

/** Amount field only. The containing resource supplies currency; no implicit FX. */
class MoneyAmountSerializer(private val currency: Currency) : KSerializer<Money> {
    override val descriptor = PrimitiveSerialDescriptor("org.cuenora.shared.MoneyAmount", PrimitiveKind.STRING)

    override fun serialize(encoder: Encoder, value: Money) {
        if (value.currency != currency) throw SerializationException("Amount currency does not match resource currency")
        val units = value.minorUnits
        val magnitude = if (units < 0) -units else units // Money excludes Long.MIN_VALUE.
        encoder.encodeString((if (units < 0) "-" else "") + decimalDigits(magnitude.toString()))
    }

    override fun deserialize(decoder: Decoder): Money {
        val text = decoder.amountString()
        if (!recordedDecimal.matches(text)) invalidAmount()
        val magnitude = minorDigits(text.removePrefix("-")).toLongOrNull() ?: invalidAmount()
        return Money(if (text.startsWith('-')) -magnitude else magnitude, currency)
    }
}

/** Arbitrary-precision projected payout, not a recorded amount or interest rate. */
object ProjectedAmountSerializer : KSerializer<ProjectedMinorUnits> {
    override val descriptor = PrimitiveSerialDescriptor("org.cuenora.shared.ProjectedAmount", PrimitiveKind.STRING)

    override fun serialize(encoder: Encoder, value: ProjectedMinorUnits) {
        encoder.encodeString(decimalDigits(value.digits))
    }

    override fun deserialize(decoder: Decoder): ProjectedMinorUnits {
        val text = decoder.amountString()
        if (!projectedDecimal.matches(text)) invalidAmount()
        return ProjectedMinorUnits(minorDigits(text))
    }
}

/** Validates OpenAPI ReportAmount without narrowing aggregate values or changing text. */
object ReportAmountSerializer : KSerializer<String> {
    override val descriptor = PrimitiveSerialDescriptor("org.cuenora.shared.ReportAmount", PrimitiveKind.STRING)

    override fun serialize(encoder: Encoder, value: String) {
        if (!reportDecimal.matches(value)) invalidAmount()
        encoder.encodeString(value)
    }

    override fun deserialize(decoder: Decoder): String {
        val text = decoder.amountString()
        if (!reportDecimal.matches(text)) invalidAmount()
        return text
    }
}
