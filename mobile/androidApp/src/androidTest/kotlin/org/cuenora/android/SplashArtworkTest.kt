package org.cuenora.android

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.AdaptiveIconDrawable
import android.graphics.drawable.ColorDrawable
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test

/** Exercise the foreground expansion and device mask, not just the drawable XML. */
@androidx.test.filters.SdkSuppress(minSdkVersion = 31)
class SplashArtworkTest {
    private fun render(size: Int, masked: Boolean, maskData: String? = null): Bitmap {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val drawable = requireNotNull(context.getDrawable(R.drawable.ic_splash))
        val bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        if (masked) {
            val adaptive = AdaptiveIconDrawable(ColorDrawable(Color.TRANSPARENT), ColorDrawable(Color.TRANSPARENT))
            adaptive.setBounds(0, 0, size, size)
            val mask = if (maskData == null) adaptive.iconMask else {
                val path = requireNotNull(androidx.core.graphics.PathParser.createPathFromPathData(maskData))
                path.transform(android.graphics.Matrix().apply { setScale(size / 100f, size / 100f) })
                path
            }
            canvas.clipPath(mask)
        }
        // AOSP AdaptiveForegroundDrawable expands foreground bounds by 1.5 before masking.
        val half = (size / ((2f / 3f) * 2)).toInt()
        drawable.setBounds(size / 2 - half, size / 2 - half, size / 2 + half, size / 2 + half)
        drawable.draw(canvas)
        return bitmap
    }

    @Test fun blueMarkHasMaskClearanceAndSurvivesWithoutClippedEdges() = assertMaskClearance(null)

    @Test fun blueMarkSurvivesInsetRoundedDeviceMask() = assertMaskClearance(
        // One UI API36 system mask: cardinal edges are inset 1% from the nominal bounds.
        "M99,50C99,6.935,77.063,1,50,1C22.935,1,1,6.935,1,50" +
            "c0,43.063,21.935,49,49,49C77.063,99,99,93.063,99,50z",
    )

    private fun assertMaskClearance(maskData: String?) {
        for (size in listOf(288, 576)) {
            val original = render(size, masked = false)
            val masked = render(size, masked = true, maskData = maskData)
            try {
                val clearance = size / 72 // Four pixels at the nominal 288px rendering size.
                for (y in 0 until size) for (x in 0 until size) {
                    val pixel = original.getPixel(x, y)
                    if (Color.alpha(pixel) > 128) {
                        assertTrue("Splash mark touches mask boundary at $x,$y for size $size",
                            x >= clearance && y >= clearance && x < size - clearance && y < size - clearance)
                        assertEquals("System mask clips splash artwork at $x,$y", pixel, masked.getPixel(x, y))
                    }
                }
            } finally { original.recycle(); masked.recycle() }
        }
    }

    @Test fun markUsesOnlyBrandBlueAndHasNoBackgroundDisc() {
        val bitmap = render(576, masked = true)
        try {
            fun bounds(predicate: (Int) -> Boolean): Rect {
                val result = Rect(bitmap.width, bitmap.height, 0, 0)
                for (y in 0 until bitmap.height) for (x in 0 until bitmap.width) {
                    if (predicate(bitmap.getPixel(x, y))) {
                        result.left = minOf(result.left, x)
                        result.top = minOf(result.top, y)
                        result.right = maxOf(result.right, x + 1)
                        result.bottom = maxOf(result.bottom, y + 1)
                    }
                }
                return result
            }
            val mark = bounds { Color.alpha(it) > 128 }
            assertTrue("Brand-blue mark must be present", mark.width() > 0 && mark.height() > 0)
            for (y in mark.top until mark.bottom) for (x in mark.left until mark.right) {
                val color = bitmap.getPixel(x, y)
                if (Color.alpha(color) > 128) {
                    // Eight-bit premultiplication/unpremultiplication loses up to 255/alpha
                    // channel units at softened edges. Opaque interior pixels must stay exact.
                    val alpha = Color.alpha(color)
                    val tolerance = if (alpha == 255) 0 else kotlin.math.ceil(255.0 / alpha).toInt()
                    for ((expected, actual) in listOf(36 to Color.red(color),
                            102 to Color.green(color), 209 to Color.blue(color))) {
                        assertTrue("Brand channel $expected became $actual at alpha $alpha",
                            kotlin.math.abs(expected - actual) <= tolerance)
                    }
                }
            }
        } finally { bitmap.recycle() }
    }
}
