package org.cuenora.android.ui

import androidx.core.splashscreen.SplashScreenViewProvider
import org.cuenora.shared.auth.AuthPhase
import org.cuenora.shared.onboarding.OnboardingStage
import kotlin.math.ceil

/** Presentation-only startup gate; it never invokes authentication or onboarding operations. */
internal class StartupSplashController {
    private var ready = false
    private var destroyed = false
    private var exitSurface: StartupSplashExitSurface? = null
    private var removal: Runnable? = null

    internal val isDestroyed: Boolean get() = destroyed
    internal val hasPendingExitRemoval: Boolean get() = removal != null

    fun shouldKeepOnScreen(): Boolean = !ready && !destroyed

    fun markReady() {
        if (!destroyed) ready = true
    }

    fun onExit(provider: SplashScreenViewProvider, durationScale: Float, nowMillis: Long) {
        onExit(
            surface = SplashScreenProviderSurface(provider),
            startMillis = provider.iconAnimationStartMillis,
            durationMillis = provider.iconAnimationDurationMillis,
            durationScale = durationScale,
            nowMillis = nowMillis,
        )
    }

    internal fun onExit(
        surface: StartupSplashExitSurface,
        startMillis: Long,
        durationMillis: Long,
        durationScale: Float,
        nowMillis: Long,
    ) {
        if (destroyed) {
            surface.remove()
            return
        }
        this.exitSurface = surface
        val delay = StartupSplashReadiness.remainingAnimationMillis(
            startMillis,
            durationMillis,
            nowMillis,
            durationScale,
        )
        val callback = Runnable {
            removal = null
            this.exitSurface = null
            surface.remove()
        }
        removal = callback
        if (delay == 0L) callback.run() else surface.postDelayed(callback, delay)
    }

    fun close() {
        destroyed = true
        removal?.let { exitSurface?.removeCallbacks(it) }
        removal = null
        exitSurface?.remove()
        exitSurface = null
    }
}

internal interface StartupSplashExitSurface {
    fun postDelayed(callback: Runnable, delayMillis: Long)
    fun removeCallbacks(callback: Runnable)
    fun remove()
}

private class SplashScreenProviderSurface(
    private val provider: SplashScreenViewProvider,
) : StartupSplashExitSurface {
    override fun postDelayed(callback: Runnable, delayMillis: Long) {
        provider.view.postDelayed(callback, delayMillis)
    }

    override fun removeCallbacks(callback: Runnable) {
        provider.view.removeCallbacks(callback)
    }

    override fun remove() {
        provider.remove()
    }
}

internal object StartupSplashReadiness {
    fun isReady(auth: AuthPhase, onboarding: OnboardingStage?, busy: Boolean): Boolean = when (auth) {
        AuthPhase.Restoring, AuthPhase.Working -> false
        AuthPhase.SignedIn, AuthPhase.Renewing -> onboarding != null &&
            onboarding != OnboardingStage.Loading && !busy
        AuthPhase.SignedOut, AuthPhase.StorageBlocked -> true
    }

    fun remainingAnimationMillis(startMillis: Long, durationMillis: Long, nowMillis: Long, durationScale: Float): Long {
        if (startMillis <= 0L || durationMillis <= 0L || durationScale <= 0f || !durationScale.isFinite()) return 0L
        val scaledDuration = ceil(durationMillis * durationScale.toDouble()).toLong()
        return (scaledDuration - (nowMillis - startMillis).coerceAtLeast(0L)).coerceAtLeast(0L)
    }
}
