package org.cuenora.android

import android.app.Activity
import android.view.WindowManager
import android.widget.DatePicker
import android.widget.EditText
import androidx.compose.runtime.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.height
import androidx.compose.ui.unit.dp
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.activity.compose.LocalActivity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.replaceText
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.*
import androidx.test.espresso.matcher.RootMatchers.isDialog
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.cuenora.android.ui.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class NativeInputUxTest {
    @get:Rule val compose = createComposeRule()

    @Test fun fieldBodyOpensCalendarAfterNumericImeResizesViewport() {
        lateinit var view: android.view.View
        var amount by mutableStateOf("")
        var changes = 0
        compose.setContent {
            view = LocalView.current
            CuenoraTheme {
                androidx.compose.foundation.layout.Column(
                    Modifier.imePadding().verticalScroll(rememberScrollState()),
                ) {
                    CuenoraField("Amount", amount, { amount = it }, Modifier.testTag("amount"),
                        decimal = true, canonicalizeDecimal = true)
                    androidx.compose.foundation.layout.Spacer(Modifier.height(700.dp))
                    CuenoraDateField("Date", "2026-11-08", { changes++ }, Modifier.testTag("date"))
                }
            }
        }
        compose.onNodeWithTag("amount").performClick().performTextReplacement("1,25")
        compose.waitUntil(10_000) {
            ViewCompat.getRootWindowInsets(view)?.isVisible(WindowInsetsCompat.Type.ime()) == true
        }
        compose.awaitStableDateViewport("date")
        compose.onNodeWithTag("date").performScrollTo()
        compose.awaitStableDateViewport("date")
        compose.onNodeWithTag("date").performTouchInput { click(center) }
        onView(isAssignableFrom(DatePicker::class.java)).inRoot(isDialog()).check(matches(isDisplayed()))
        assertEquals("1.25", amount)
        assertEquals(0, changes)
    }

    @Test fun nativeDateDialogsInheritSecureScreenPolicy() {
        lateinit var activity: Activity
        compose.setContent {
            activity = checkNotNull(LocalActivity.current)
            CuenoraTheme { CuenoraDateField("Date", "2026-11-08", {}) }
        }
        compose.runOnIdle { activity.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE) }
        try {
            compose.onNodeWithContentDescription("Choose date").performSemanticsAction(SemanticsActions.OnClick) { it() }
            onView(isAssignableFrom(DatePicker::class.java)).inRoot(isDialog()).check { view, error ->
                if (error != null) throw error
                val flags = (view.rootView.layoutParams as WindowManager.LayoutParams).flags
                assertTrue("Calendar must protect a secure screen", flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
            }
            onView(withText(android.R.string.cancel)).inRoot(isDialog()).perform(click())
            compose.onNodeWithContentDescription("Enter date manually").performSemanticsAction(SemanticsActions.OnClick) { it() }
            onView(isAssignableFrom(EditText::class.java)).inRoot(isDialog()).check { view, error ->
                if (error != null) throw error
                val flags = (view.rootView.layoutParams as WindowManager.LayoutParams).flags
                assertTrue("Manual date entry must protect a secure screen", flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
            }
        } finally {
            compose.runOnIdle { activity.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE) }
        }
    }

    @Test fun actualDateFieldBodyTouchOpensPickerWithoutChangingDraft() {
        var changes = 0
        compose.setContent {
            CuenoraTheme { CuenoraDateField("Date", "2026-11-08", { changes++ }, Modifier.testTag("date")) }
        }
        compose.onNodeWithTag("date").performTouchInput { click(center) }
        onView(isAssignableFrom(DatePicker::class.java)).check(matches(isDisplayed()))
        assertEquals(0, changes)
    }

    @Test fun calendarIconRetainsActivityWindowAndExistingPre1900Date() {
        var value by mutableStateOf("1800-02-03")
        var changes = 0
        compose.setContent {
            CuenoraTheme { CuenoraDateField("Date", value, { value = it; changes++ }) }
        }
        compose.onNodeWithContentDescription("Choose date").performClick()
        assertEquals(0, changes)
        onView(isAssignableFrom(DatePicker::class.java)).check { view, error ->
            if (error != null) throw error
            val picker = view as DatePicker
            assertEquals(1800, picker.year)
            assertEquals(1, picker.month)
            assertEquals(3, picker.dayOfMonth)
        }
        onView(withText(android.R.string.ok)).perform(click())
        assertEquals("1800-02-03", value)
        assertEquals(1, changes)
    }

    @Test fun exactManualEntryPreservesDatesOutsidePlatformCalendarChronology() {
        var value by mutableStateOf("2026-11-08")
        compose.setContent { CuenoraTheme { CuenoraDateField("Date", value, { value = it }) } }
        compose.onNodeWithContentDescription("Enter date manually").performClick()
        onView(isAssignableFrom(EditText::class.java)).inRoot(isDialog()).perform(replaceText("0001-01-01"))
        onView(withText(android.R.string.ok)).perform(click())
        assertEquals("0001-01-01", value)
        compose.onNodeWithContentDescription("Enter date manually").performSemanticsAction(SemanticsActions.OnClick) { it() }
        onView(isAssignableFrom(EditText::class.java)).inRoot(isDialog()).perform(replaceText("1582-10-10"))
        onView(withText(android.R.string.cancel)).perform(click())
        assertEquals("0001-01-01", value)
    }

    @Test fun disabledDateBodyAndIconsCannotMutate() {
        var changes = 0
        compose.setContent {
            CuenoraTheme { CuenoraDateField("Date", "2026-11-08", { changes++ }, Modifier.testTag("date"), enabled = false) }
        }
        compose.onNodeWithTag("date").assertIsNotEnabled().performTouchInput { click() }
        compose.onNodeWithContentDescription("Choose date").assertIsNotEnabled().performTouchInput { click() }
        compose.onNodeWithContentDescription("Enter date manually").assertIsNotEnabled()
        assertEquals(0, changes)
    }

    @Test fun numericNormalizationDoesNotReinterpretMalformedOrFreeTextValues() {
        var number by mutableStateOf("")
        var text by mutableStateOf("")
        compose.setContent {
            CuenoraTheme { androidx.compose.foundation.layout.Column {
                CuenoraField("Number", number, { number = it }, Modifier.testTag("number"), decimal = true, canonicalizeDecimal = true)
                CuenoraField("Description", text, { text = it }, Modifier.testTag("text"))
            } }
        }
        listOf("1,2,3", "1.2.3", "1,234.56").forEach {
            compose.onNodeWithTag("number").performTextReplacement(it)
            assertEquals(it, number)
        }
        compose.onNodeWithTag("number").performTextReplacement("12,34")
        assertEquals("12.34", number)
        compose.onNodeWithTag("text").performTextReplacement("12,34")
        assertEquals("12,34", text)
    }
}
