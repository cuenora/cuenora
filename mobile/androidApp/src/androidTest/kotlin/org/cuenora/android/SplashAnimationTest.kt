package org.cuenora.android

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.AnimatedVectorDrawable
import android.graphics.drawable.Animatable2
import android.graphics.drawable.Drawable
import android.view.ViewGroup
import android.widget.ImageView
import androidx.test.core.app.ActivityScenario
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Confirm the native drawable actually starts and reaches its complete frame in a window. */
@androidx.test.filters.SdkSuppress(minSdkVersion = 31)
class SplashAnimationTest {
    @Test fun attachedSplashAnimationCompletes() {
        val ended = CountDownLatch(1)
        val started = CountDownLatch(1)
        val scenario = ActivityScenario.launch(FoundationActivity::class.java)
        try {
            scenario.onActivity { activity ->
                val image = ImageView(activity)
                (activity.window.decorView as ViewGroup).addView(
                    image,
                    ViewGroup.LayoutParams(432, 432),
                )
                val drawable = activity.getDrawable(R.drawable.ic_splash) as? AnimatedVectorDrawable
                assertNotNull("Splash resource must resolve as AnimatedVectorDrawable", drawable)
                image.setImageDrawable(drawable)
                drawable!!.registerAnimationCallback(object : Animatable2.AnimationCallback() {
                    override fun onAnimationStart(drawable: Drawable) = started.countDown()
                    override fun onAnimationEnd(drawable: Drawable) = ended.countDown()
                })
                image.post { drawable.start() }
            }
            assertTrue("Attached splash animation should start", started.await(2, TimeUnit.SECONDS))
            assertTrue("Splash animation should reach its full frame", ended.await(2, TimeUnit.SECONDS))
            scenario.onActivity { activity ->
                val image = (activity.window.decorView as ViewGroup).let { it.getChildAt(it.childCount - 1) } as ImageView
                val completed = Bitmap.createBitmap(432, 432, Bitmap.Config.ARGB_8888)
                val fallback = Bitmap.createBitmap(432, 432, Bitmap.Config.ARGB_8888)
                try {
                    image.drawable.setBounds(0, 0, 432, 432)
                    image.drawable.draw(Canvas(completed))
                    requireNotNull(activity.getDrawable(R.drawable.ic_splash)).apply {
                        setBounds(0, 0, 432, 432)
                        draw(Canvas(fallback))
                    }
                    assertTrue("Animation must finish on the complete static mark", completed.sameAs(fallback))
                } finally {
                    completed.recycle()
                    fallback.recycle()
                }
            }
        } finally {
            scenario.close()
        }
    }
}
