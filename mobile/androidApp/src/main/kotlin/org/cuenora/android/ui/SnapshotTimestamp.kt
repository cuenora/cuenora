package org.cuenora.android.ui

import java.text.DateFormat
import java.text.ParsePosition
import java.text.SimpleDateFormat
import java.util.Date
import java.util.GregorianCalendar
import java.util.Locale
import java.util.TimeZone

/** Display-only conversion: never changes stored audit instants, business dates or financial today. */
internal fun formatSnapshotTimestamp(value: String, profileZone: String?, locale: Locale): String? {
    val zoneId = profileZone ?: "UTC"
    if (zoneId !in TimeZone.getAvailableIDs()) return null
    val match = Regex("([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2})(?:\\.[0-9]{1,9})?Z")
        .matchEntire(value) ?: return null
    // The UI displays minutes; subsecond precision remains unchanged in the original server value.
    val normalized = match.groupValues[1] + "Z"
    val utc = TimeZone.getTimeZone("UTC")
    val parser = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.ROOT).apply {
        calendar = GregorianCalendar(utc, Locale.ROOT).apply {
            gregorianChange = Date(Long.MIN_VALUE)
            isLenient = false
        }
        isLenient = false
    }
    val position = ParsePosition(0)
    val instant = parser.parse(normalized, position) ?: return null
    if (position.index != normalized.length) return null
    val formatted = DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT, locale).apply {
        timeZone = TimeZone.getTimeZone(zoneId)
    }.format(instant)
    return "$formatted · $zoneId"
}
