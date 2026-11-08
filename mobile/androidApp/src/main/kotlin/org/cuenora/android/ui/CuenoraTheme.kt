package org.cuenora.android.ui

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import org.cuenora.android.R

internal val DisplayFont = FontFamily(Font(R.font.bodoni_moda, FontWeight.Medium))
internal val UiFont = FontFamily(
    Font(R.font.ibm_plex_mono_regular),
    Font(R.font.ibm_plex_mono_medium, FontWeight.Medium),
)

internal val LightColors = lightColorScheme(
    primary = Color(0xFF2466D1), onPrimary = Color.White,
    primaryContainer = Color(0xFFC2D5F4), onPrimaryContainer = Color(0xFF0B1E3D),
    secondary = Color(0xFF18448B), onSecondary = Color.White,
    secondaryContainer = Color(0xFFC2D5F4), onSecondaryContainer = Color(0xFF0B1E3D),
    tertiary = Color(0xFF18448B), onTertiary = Color.White,
    background = Color.White, onBackground = Color(0xFF0B1E3D),
    surface = Color.White, onSurface = Color(0xFF0B1E3D),
    surfaceVariant = Color(0xFFE9F0FB), onSurfaceVariant = Color(0xFF18448B),
    surfaceContainer = Color(0xFFE9F0FB), surfaceContainerLow = Color.White,
    surfaceContainerHigh = Color(0xFFE9F0FB),
    outline = Color(0xFF1F57B2), outlineVariant = Color(0xFFC2D5F4),
    error = Color(0xFF9B2440), onError = Color.White,
    errorContainer = Color(0xFFFFF0F3), onErrorContainer = Color(0xFF9B2440),
)
internal val DarkColors = darkColorScheme(
    primary = Color(0xFF9BBBED), onPrimary = Color(0xFF040B16),
    primaryContainer = Color(0xFF143770), onPrimaryContainer = Color(0xFFE9F0FB),
    secondary = Color(0xFF9BBBED), onSecondary = Color(0xFF040B16),
    secondaryContainer = Color(0xFF143770), onSecondaryContainer = Color(0xFFE9F0FB),
    tertiary = Color(0xFF9BBBED), onTertiary = Color(0xFF040B16),
    background = Color(0xFF040B16), onBackground = Color(0xFFE9F0FB),
    surface = Color(0xFF0B1E3D), onSurface = Color(0xFFE9F0FB),
    surfaceVariant = Color(0xFF143770), onSurfaceVariant = Color(0xFF9BBBED),
    surfaceContainer = Color(0xFF0B1E3D), surfaceContainerLow = Color(0xFF0B1E3D),
    surfaceContainerHigh = Color(0xFF143770),
    outline = Color(0xFF74A0E7), outlineVariant = Color(0xFF18448B),
    error = Color(0xFFFFB2C1), onError = Color(0xFF040B16),
    errorContainer = Color(0xFFFFF0F3), onErrorContainer = Color(0xFF9B2440),
)

private fun uiStyle(size: Int, weight: FontWeight = FontWeight.Normal) = TextStyle(
    fontFamily = UiFont, fontWeight = weight, fontSize = size.sp, lineHeight = (size * 1.5).sp,
)
private fun heading(size: Int) = TextStyle(
    fontFamily = DisplayFont, fontWeight = FontWeight.Medium,
    fontSize = size.sp, lineHeight = (size * 1.2).sp,
)

@Composable
fun CuenoraTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    // Deliberately retain brand colors instead of wallpaper-derived dynamic colors.
    val view = LocalView.current
    val context = LocalContext.current
    val window = remember(context) { context.findActivity()?.window }
    SideEffect {
        window?.let { WindowCompat.getInsetsController(it, view).isAppearanceLightStatusBars = !darkTheme }
    }
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = Typography(
            displayLarge = heading(44), displayMedium = heading(44), displaySmall = heading(32),
            headlineLarge = heading(32), headlineMedium = heading(32), headlineSmall = heading(24),
            titleLarge = heading(24), titleMedium = uiStyle(18, FontWeight.Medium),
            titleSmall = uiStyle(16, FontWeight.Medium),
            bodyLarge = uiStyle(16), bodyMedium = uiStyle(14), bodySmall = uiStyle(14),
            labelLarge = uiStyle(16, FontWeight.Medium),
            labelMedium = uiStyle(14, FontWeight.Medium), labelSmall = uiStyle(13),
        ),
        shapes = Shapes(
            extraSmall = RoundedCornerShape(10.dp), small = RoundedCornerShape(10.dp),
            medium = RoundedCornerShape(12.dp), large = RoundedCornerShape(12.dp),
            extraLarge = RoundedCornerShape(12.dp),
        ),
        content = content,
    )
}

private tailrec fun Context.findActivity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.findActivity()
    else -> null
}
