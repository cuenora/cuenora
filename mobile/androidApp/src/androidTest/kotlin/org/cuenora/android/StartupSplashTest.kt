package org.cuenora.android

import android.content.Intent
import android.os.SystemClock
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.runner.lifecycle.ActivityLifecycleCallback
import androidx.test.runner.lifecycle.ActivityLifecycleMonitorRegistry
import androidx.test.runner.lifecycle.Stage
import org.cuenora.android.ui.StartupSplashController
import org.cuenora.android.ui.StartupSplashExitSurface
import org.cuenora.android.ui.StartupSplashReadiness
import org.cuenora.shared.auth.AuthPhase
import org.cuenora.shared.onboarding.OnboardingStage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

@RunWith(AndroidJUnit4::class)
class StartupSplashTest {
    @Test fun terminalAuthenticationOutcomesReleaseSplashToExistingRecoveryUi() {
        assertTrue(StartupSplashReadiness.isReady(AuthPhase.SignedOut, null, busy = false))
        assertTrue(StartupSplashReadiness.isReady(AuthPhase.StorageBlocked, null, busy = false))
        assertFalse(StartupSplashReadiness.isReady(AuthPhase.Working, null, busy = false))
    }

    @Test fun signedInStartupWaitsForInitialProfileAndAccountsCheck() {
        assertFalse(StartupSplashReadiness.isReady(AuthPhase.Restoring, null, busy = false))
        assertFalse(StartupSplashReadiness.isReady(AuthPhase.SignedIn, OnboardingStage.Loading, busy = false))
        assertFalse(StartupSplashReadiness.isReady(AuthPhase.Renewing, OnboardingStage.Existing, busy = true))
        for (stage in listOf(OnboardingStage.Verify, OnboardingStage.Existing, OnboardingStage.Empty,
                OnboardingStage.Uncertain, OnboardingStage.Failed)) {
            assertTrue("$stage should reveal its existing UI", StartupSplashReadiness.isReady(
                AuthPhase.SignedIn, stage, busy = false))
        }
    }

    @Test fun animationHoldUsesPlatformMetadataAndDurationScale() {
        assertEquals(0L, StartupSplashReadiness.remainingAnimationMillis(0L, 0L, 1_000L, 1f))
        assertEquals(400L, StartupSplashReadiness.remainingAnimationMillis(1_000L, 900L, 1_500L, 1f))
        assertEquals(1_300L, StartupSplashReadiness.remainingAnimationMillis(1_000L, 900L, 1_500L, 2f))
        assertEquals(0L, StartupSplashReadiness.remainingAnimationMillis(1_000L, 900L, 1_500L, 0f))
    }

    @Test fun syntheticActivityBlocksContentUntilReadyThenInvokesRealSplashCallback() {
        val activity = launchSyntheticActivity()
        try {
            InstrumentationRegistry.getInstrumentation().runOnMainSync {
                assertTrue(activity.splashController.shouldKeepOnScreen())
            }
            SystemClock.sleep(150)
            assertEquals("blocked pre-draw must not render content", 1L, activity.firstContentDraw.count)
            InstrumentationRegistry.getInstrumentation().runOnMainSync {
                activity.releaseSplash()
                assertFalse(activity.splashController.shouldKeepOnScreen())
            }
            assertTrue("ready state should advance the compat/platform splash", activity.exitCallback.await(3, TimeUnit.SECONDS))
            assertTrue("content should render after readiness", activity.firstContentDraw.await(3, TimeUnit.SECONDS))
        } finally {
            finishActivity(activity)
        }
    }

    @Test fun destroyingSyntheticActivityClosesAStillBlockedGate() {
        val activity = launchSyntheticActivity()
        val controller = activity.splashController
        assertTrue(controller.shouldKeepOnScreen())
        finishActivity(activity)
        assertTrue(controller.isDestroyed)
        assertFalse(controller.shouldKeepOnScreen())
    }

    @Test fun destroyingSyntheticActivityCancelsPendingAnimationRemoval() {
        val activity = launchSyntheticActivity()
        val controller = activity.splashController
        val surface = FakeExitSurface()
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            controller.onExit(surface, startMillis = 1_000L, durationMillis = 360L, durationScale = 1f, nowMillis = 1_000L)
        }
        assertEquals(360L, surface.scheduledDelayMillis)
        assertTrue(controller.hasPendingExitRemoval)
        finishActivity(activity)
        assertTrue(controller.isDestroyed)
        assertTrue(surface.cancelled)
        assertTrue(surface.removed)
        assertFalse(controller.hasPendingExitRemoval)
    }

    private fun launchSyntheticActivity(): StartupTestActivity {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val resumed = CountDownLatch(1)
        val activity = AtomicReference<StartupTestActivity>()
        val callback = ActivityLifecycleCallback { candidate, stage ->
            if (candidate is StartupTestActivity && stage == Stage.RESUMED) {
                activity.set(candidate)
                resumed.countDown()
            }
        }
        val monitor = ActivityLifecycleMonitorRegistry.getInstance()
        monitor.addLifecycleCallback(callback)
        try {
            instrumentation.runOnMainSync {
                val context = instrumentation.targetContext
                context.startActivity(Intent(context, StartupTestActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            }
            assertTrue("synthetic host should resume without waiting for a draw", resumed.await(3, TimeUnit.SECONDS))
            return activity.get()
        } finally {
            monitor.removeLifecycleCallback(callback)
        }
    }

    private fun finishActivity(activity: StartupTestActivity) {
        val destroyed = CountDownLatch(1)
        val callback = ActivityLifecycleCallback { candidate, stage ->
            if (candidate === activity && stage == Stage.DESTROYED) destroyed.countDown()
        }
        val monitor = ActivityLifecycleMonitorRegistry.getInstance()
        monitor.addLifecycleCallback(callback)
        try {
            InstrumentationRegistry.getInstrumentation().runOnMainSync { activity.finish() }
            assertTrue("synthetic host should be destroyed", destroyed.await(3, TimeUnit.SECONDS))
        } finally {
            monitor.removeLifecycleCallback(callback)
        }
    }

    private class FakeExitSurface : StartupSplashExitSurface {
        var scheduledDelayMillis = -1L
        var pending: Runnable? = null
        var cancelled = false
        var removed = false

        override fun postDelayed(callback: Runnable, delayMillis: Long) {
            pending = callback
            scheduledDelayMillis = delayMillis
        }

        override fun removeCallbacks(callback: Runnable) {
            if (pending === callback) pending = null
            cancelled = true
        }

        override fun remove() {
            removed = true
        }
    }
}
