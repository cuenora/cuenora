package org.cuenora.android

import org.cuenora.android.ui.formatSnapshotTimestamp
import org.junit.Assert.*
import org.junit.Test
import java.util.Locale
import java.util.TimeZone

class SnapshotDateFormatterTest {
    @Test fun confirmedProfileZoneUsesTheInstantsWinterAndSummerOffsets() {
        val winter = formatSnapshotTimestamp("2026-11-15T10:22:18.392332Z", "Europe/Stockholm", Locale.UK)!!
        val summer = formatSnapshotTimestamp("2026-07-15T10:22:18.392332Z", "Europe/Stockholm", Locale.UK)!!
        assertTrue(winter.contains("15 Nov 2026"))
        assertTrue(winter.contains("11:22"))
        assertTrue(summer.contains("12:22"))
        assertTrue(winter.endsWith("Europe/Stockholm"))
        assertFalse(winter.contains("T10:22"))
    }

    @Test fun profileZoneControlsDayBoundaryEvenWhenDeviceZoneDiffers() {
        val previous = TimeZone.getDefault()
        try {
            TimeZone.setDefault(TimeZone.getTimeZone("America/Los_Angeles"))
            val value = formatSnapshotTimestamp("2026-11-15T23:30:00Z", "Europe/Stockholm", Locale.UK)!!
            assertTrue(value.contains("16 Nov 2026"))
            assertTrue(value.contains("00:30"))
        } finally { TimeZone.setDefault(previous) }
    }

    @Test fun localeIsRespectedAndMissingZoneIsExplicitlyUtc() {
        val instant = "2026-11-15T10:22:18.392332Z"
        val english = formatSnapshotTimestamp(instant, null, Locale.US)!!
        val french = formatSnapshotTimestamp(instant, null, Locale.FRANCE)!!
        assertNotEquals(english, french)
        assertTrue(english.endsWith("UTC"))
        assertTrue(english.contains("10:22"))
    }

    @Test fun invalidZoneOrInstantNeverSilentlyFallsBackOrExposesRawInput() {
        assertNull(formatSnapshotTimestamp("2026-11-15T10:22:18Z", "Invalid/Zone", Locale.UK))
        assertNull(formatSnapshotTimestamp("2026-02-30T10:22:18Z", "UTC", Locale.UK))
        assertNull(formatSnapshotTimestamp("2026-11-15T10:22:18Z extra", "UTC", Locale.UK))
    }
}
