package org.cuenora.android.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

internal fun Modifier.cuenoraFocus(): Modifier = composed {
    var focused by remember { mutableStateOf(false) }
    val color = MaterialTheme.colorScheme.secondary
    onFocusChanged { focused = it.hasFocus }.drawWithContent {
        drawContent()
        if (focused) {
            val inset = 1.dp.toPx()
            drawRoundRect(
                color, topLeft = Offset(inset, inset),
                size = Size(size.width - 2 * inset, size.height - 2 * inset),
                cornerRadius = CornerRadius(14.dp.toPx()), style = Stroke(2.dp.toPx()),
            )
        }
    }.padding(4.dp) // Allocate the 2dp ring and 2dp gap without overlapping neighbors.
}

@Composable
fun CuenoraAction(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    unavailableReason: String? = null,
    busy: Boolean = false,
) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(
            onClick = onClick, enabled = !busy && unavailableReason == null,
            modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).cuenoraFocus(),
            colors = ButtonDefaults.buttonColors(
                disabledContainerColor = MaterialTheme.colorScheme.surfaceVariant,
                disabledContentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            ),
        ) { Text(if (busy) "Saving…" else label) }
        if (busy || unavailableReason != null) {
            Text(
                if (busy) "Submission in progress. Please wait." else unavailableReason.orEmpty(),
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
        }
    }
}

@Composable
fun CuenoraField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    supportingText: String = "",
    error: String? = null,
) {
    OutlinedTextField(
        value = value, onValueChange = onValueChange,
        label = { Text(label) }, isError = error != null,
        supportingText = { Text(error?.let { "Error: $it" } ?: supportingText) },
        modifier = modifier.fillMaxWidth().heightIn(min = 56.dp).cuenoraFocus(),
        shape = RoundedCornerShape(10.dp),
    )
}

enum class NoticeTone(val label: String, internal val foreground: Color, internal val background: Color) {
    Information("Information", Color(0xFF18448B), Color(0xFFE9F0FB)),
    Success("Confirmed", Color(0xFF16633F), Color(0xFFE9F6EE)),
    Attention("Attention", Color(0xFF7B4A00), Color(0xFFFFF3DA)),
    Error("Error", Color(0xFF9B2440), Color(0xFFFFF0F3)),
}

@Composable
fun CuenoraNotice(title: String, message: String, tone: NoticeTone = NoticeTone.Information) {
    Surface(color = tone.background, contentColor = tone.foreground, shape = RoundedCornerShape(12.dp)) {
        Row(Modifier.fillMaxWidth().padding(16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Icon(
                when (tone) {
                    NoticeTone.Information -> Icons.Default.Info
                    NoticeTone.Success -> Icons.Default.CheckCircle
                    else -> Icons.Default.Warning
                },
                contentDescription = null,
            )
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("${tone.label} · $title", style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.semantics { heading() })
                Text(message)
            }
        }
    }
}

@Composable
fun CuenoraAmount(meaning: String, currencyCode: String, signedAmount: String) {
    // Display preformatted exact values from presentation logic; never parse to binary floating point.
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(meaning, style = MaterialTheme.typography.bodyMedium)
        Text("$currencyCode $signedAmount", fontFamily = UiFont, fontWeight = FontWeight.Medium,
            fontSize = 24.sp, lineHeight = 36.sp)
    }
}

@Composable
fun CuenoraStatus(label: String) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
        shape = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
    ) { Text(label, Modifier.padding(8.dp), style = MaterialTheme.typography.bodyMedium) }
}

@Composable
fun CuenoraLoading(message: String) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.semantics {
        contentDescription = message
        liveRegion = LiveRegionMode.Polite
    }) {
        CircularProgressIndicator()
        Text(message)
    }
}

@Composable
fun CuenoraDiscardDialog(onKeepEditing: () -> Unit, onDiscard: () -> Unit) {
    val safeAction = remember { FocusRequester() }
    AlertDialog(
        onDismissRequest = onKeepEditing,
        title = { Text("Discard changes?") },
        text = { Text("Your unsubmitted changes will be lost. Recorded activity is not affected.") },
        confirmButton = {
            val windowFocused = LocalWindowInfo.current.isWindowFocused
            LaunchedEffect(windowFocused) {
                if (windowFocused) {
                    withFrameNanos { }
                    safeAction.requestFocus()
                }
            }
            TextButton(onClick = onKeepEditing, modifier = Modifier.focusRequester(safeAction).focusProperties { canFocus = true }.cuenoraFocus()) {
                Text("Keep editing")
            }
        },
        dismissButton = {
            TextButton(onClick = onDiscard, modifier = Modifier.cuenoraFocus()) {
                Text("Discard", color = MaterialTheme.colorScheme.error)
            }
        },
    )
}
