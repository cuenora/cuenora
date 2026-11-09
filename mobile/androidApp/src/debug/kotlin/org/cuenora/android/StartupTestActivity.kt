package org.cuenora.android

import android.os.Bundle
import android.graphics.Canvas
import android.provider.Settings
import android.view.View
import androidx.activity.ComponentActivity
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import org.cuenora.android.ui.StartupSplashController
import java.util.concurrent.CountDownLatch

/** Synthetic splash host for instrumentation; it never creates an auth session or backend client. */
class StartupTestActivity : ComponentActivity() {
    internal lateinit var splashController: StartupSplashController
        private set
    internal val exitCallback = CountDownLatch(1)
    internal val firstContentDraw = CountDownLatch(1)

    override fun onCreate(savedInstanceState: Bundle?) {
        splashController = StartupSplashController()
        runCatching {
            installSplashScreen().apply {
                setKeepOnScreenCondition(splashController::shouldKeepOnScreen)
                setOnExitAnimationListener { provider ->
                    exitCallback.countDown()
                    val scale = runCatching {
                        Settings.Global.getFloat(contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f)
                    }.getOrDefault(1f)
                    splashController.onExit(provider, scale, System.currentTimeMillis())
                }
            }
        }.onFailure {
            splashController.markReady()
        }
        super.onCreate(savedInstanceState)
        setContentView(object : View(this) {
            override fun onDraw(canvas: Canvas) {
                super.onDraw(canvas)
                firstContentDraw.countDown()
            }
        })
    }

    internal fun releaseSplash() = splashController.markReady()

    override fun onDestroy() {
        splashController.close()
        super.onDestroy()
    }
}
