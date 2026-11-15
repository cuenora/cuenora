package org.cuenora.android

import android.view.View
import android.graphics.Rect
import android.widget.DatePicker
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.semantics.SemanticsActions
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.test.runner.lifecycle.ActivityLifecycleMonitorRegistry
import androidx.test.runner.lifecycle.Stage
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withText
import org.hamcrest.Matcher
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

internal fun ComposeContentTestRule.selectNativeDate(tag: String, year: Int, month: Int, day: Int) {
    openNativeDate(tag)
    onView(isAssignableFrom(DatePicker::class.java)).perform(SetNativeDate(year, month - 1, day))
    onView(withText(android.R.string.ok)).check(matches(isDisplayed())).perform(click())
}

internal fun ComposeContentTestRule.clearNativeDate(tag: String) {
    openNativeDate(tag)
    onView(withText("Clear")).perform(click())
}

private fun ComposeContentTestRule.openNativeDate(tag: String) {
    // Set up a date through its public calendar action, independent of IME animation coordinates.
    // Actual field-body touch, including a visible IME, is covered separately in NativeInputUxTest.
    onNodeWithTag(tag).performScrollTo()
    onNode(hasContentDescription("Choose date") and hasAnyAncestor(hasTestTag(tag)), useUnmergedTree = true)
        .performSemanticsAction(SemanticsActions.OnClick) { it() }
}

internal fun ComposeContentTestRule.awaitStableDateViewport(tag: String) {
    val node = onNodeWithTag(tag).fetchSemanticsNode()
    val completed = CountDownLatch(1)
    var stable = false
    lateinit var root: View
    lateinit var callback: Runnable
    runOnIdle {
        val activity = ActivityLifecycleMonitorRegistry.getInstance().getActivitiesInStage(Stage.RESUMED).single()
        root = activity.window.decorView
        var previous = ""
        var repeats = 0
        var frames = 0
        callback = object : Runnable {
            override fun run() {
                val visible = Rect().also(root::getWindowVisibleDisplayFrame)
                val insets = ViewCompat.getRootWindowInsets(root)
                val geometry = "${insets?.isVisible(WindowInsetsCompat.Type.ime())}:" +
                    "${insets?.getInsets(WindowInsetsCompat.Type.ime())}:$visible:${node.boundsInRoot}"
                repeats = if (geometry == previous && root.hasWindowFocus()) repeats + 1 else 0
                previous = geometry
                stable = repeats >= 5
                frames++
                if (stable || frames >= 120) completed.countDown() else root.postOnAnimation(this)
            }
        }
        root.postOnAnimation(callback)
    }
    val finished = completed.await(10, TimeUnit.SECONDS)
    runOnIdle { root.removeCallbacks(callback) }
    check(finished && stable) { "Date input viewport did not stabilize before touch." }
    waitForIdle()
}

private class SetNativeDate(private val year: Int, private val month: Int, private val day: Int) :
    androidx.test.espresso.ViewAction {
    override fun getConstraints(): Matcher<View> = isAssignableFrom(DatePicker::class.java)
    override fun getDescription(): String = "set the native Gregorian date to $year-$month-$day"
    override fun perform(uiController: androidx.test.espresso.UiController, view: View) {
        (view as DatePicker).updateDate(year, month, day)
    }
}
