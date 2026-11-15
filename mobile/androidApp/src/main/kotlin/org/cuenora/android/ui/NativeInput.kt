package org.cuenora.android.ui

import android.app.DatePickerDialog
import android.app.AlertDialog
import android.app.Activity
import android.app.Dialog
import android.content.Context
import android.content.ContextWrapper
import android.content.DialogInterface
import android.text.InputType
import android.view.WindowManager
import android.widget.EditText
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import java.util.Calendar
import java.util.Date
import java.util.GregorianCalendar
import java.util.Locale
import java.util.TimeZone

/** Converts only a lone decimal comma; malformed/mixed separators remain invalid for shared validation. */
internal fun canonicalDecimalInput(value: String): String =
    if (value.count { it == ',' } == 1 && '.' !in value) value.replace(',', '.') else value

@Composable
fun CuenoraDateField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    supportingText: String = "",
    error: String? = null,
    enabled: Boolean = true,
    optional: Boolean = false,
) {
    val context = LocalContext.current
    val interactions = remember { MutableInteractionSource() }
    // Platform dialogs outlive the composition that opened them. Use current draft handlers.
    val currentValue = rememberUpdatedState(value)
    val currentChange = rememberUpdatedState(onValueChange)
    val currentEnabled = rememberUpdatedState(enabled)
    val currentOptional = rememberUpdatedState(optional)
    fun enterDate() {
        if (!currentEnabled.value) return
        // The platform calendar may not represent every proleptic Gregorian date.
        // Keep the prior exact-date entry route available; existing review validation still applies.
        val input = EditText(context).apply {
            inputType = InputType.TYPE_CLASS_DATETIME or InputType.TYPE_DATETIME_VARIATION_DATE
            setSingleLine(true)
            hint = "YYYY-MM-DD"
            contentDescription = label
            setText(currentValue.value)
        }
        AlertDialog.Builder(context).setTitle(label).setView(input)
            .setPositiveButton(android.R.string.ok) { _, _ -> if (currentEnabled.value) currentChange.value(input.text.toString()) }
            .setNegativeButton(android.R.string.cancel, null).create().apply {
                inheritScreenProtection(context)
                show()
            }
    }
    fun openPicker() {
        if (!currentEnabled.value) return
        val initial = dateParts(currentValue.value) ?: GregorianCalendar(TimeZone.getDefault(), Locale.getDefault()).apply {
            gregorianChange = Date(Long.MIN_VALUE)
        }.let {
            DateParts(it.get(Calendar.YEAR), it.get(Calendar.MONTH), it.get(Calendar.DAY_OF_MONTH))
        }
        val picker = DatePickerDialog(context, { _, year, month, day ->
            if (currentEnabled.value) currentChange.value(formatDate(year, month, day))
        }, initial.year, initial.month, initial.day)
        picker.datePicker.minDate = calendarMillis(1, Calendar.JANUARY, 1)
        picker.datePicker.maxDate = calendarMillis(9999, Calendar.DECEMBER, 31)
        picker.datePicker.updateDate(initial.year, initial.month, initial.day)
        if (DateParts(picker.datePicker.year, picker.datePicker.month, picker.datePicker.dayOfMonth) != initial) {
            enterDate()
            return
        }
        if (currentOptional.value) {
            picker.setButton(DialogInterface.BUTTON_NEUTRAL, "Clear") { _, _ -> if (currentEnabled.value) currentChange.value("") }
        }
        picker.inheritScreenProtection(context)
        picker.show()
    }
    val latestOpenPicker = rememberUpdatedState(::openPicker)
    LaunchedEffect(interactions) {
        interactions.interactions.collect { interaction ->
            if (interaction is PressInteraction.Release) latestOpenPicker.value()
        }
    }
    OutlinedTextField(
        value = value,
        onValueChange = {},
        readOnly = true,
        enabled = enabled,
        singleLine = true,
        interactionSource = interactions,
        label = { Text(label) },
        isError = error != null,
        supportingText = when {
            error != null -> ({ Text("Error: $error") })
            supportingText.isNotEmpty() -> ({ Text(supportingText) })
            else -> null
        },
        trailingIcon = {
            Row {
                IconButton(onClick = ::enterDate, enabled = enabled,
                    modifier = Modifier.semantics { contentDescription = "Enter date manually" }) {
                    Icon(Icons.Default.Edit, contentDescription = null)
                }
                IconButton(onClick = ::openPicker, enabled = enabled,
                    modifier = Modifier.semantics { contentDescription = "Choose date" }) {
                    Icon(Icons.Default.DateRange, contentDescription = null)
                }
            }
        },
        modifier = modifier.fillMaxWidth().heightIn(min = 56.dp).cuenoraFocus()
            .semantics {
                if (enabled) onClick(label = "Choose date") { openPicker(); true }
            },
    )
}

private fun Dialog.inheritScreenProtection(context: Context) {
    var owner = context
    while (owner is ContextWrapper) {
        if (owner is Activity) {
            if (owner.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE != 0) {
                window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            }
            return
        }
        val base = owner.baseContext
        if (base === owner) return
        owner = base
    }
}

private data class DateParts(val year: Int, val month: Int, val day: Int)

private fun dateParts(value: String): DateParts? {
    if (!Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}").matches(value)) return null
    val year = value.substring(0, 4).toIntOrNull() ?: return null
    val month = value.substring(5, 7).toIntOrNull() ?: return null
    val day = value.substring(8, 10).toIntOrNull() ?: return null
    val calendar = GregorianCalendar(TimeZone.getDefault(), Locale.getDefault()).apply {
        gregorianChange = Date(Long.MIN_VALUE)
        isLenient = false
        clear()
        set(year, month - 1, day, 12, 0, 0)
    }
    return runCatching { calendar.time; DateParts(year, month - 1, day) }.getOrNull()
}

private fun calendarMillis(year: Int, month: Int, day: Int): Long =
    GregorianCalendar(TimeZone.getDefault(), Locale.ROOT).apply {
        gregorianChange = Date(Long.MIN_VALUE)
        clear()
        set(year, month, day, 12, 0, 0)
    }.timeInMillis

private fun formatDate(year: Int, month: Int, day: Int): String =
    String.format(Locale.ROOT, "%04d-%02d-%02d", year, month + 1, day)
