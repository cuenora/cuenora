package org.cuenora.android

import android.app.Application
import kotlinx.coroutines.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.auth.HttpAuthRepository
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.session.AndroidSessionStore
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.IdempotencyKey
import java.util.Calendar
import java.util.TimeZone
import java.util.Locale
import java.util.UUID

class CuenoraApplication : Application() {
    // Lazy: loading the app/test process alone must not construct transport or read credentials.
    val authentication by lazy { NativeAuthentication(this) }
}

/** Process lifetime, not Activity lifetime: recreation cannot race two refresh coordinators. */
class NativeAuthentication(application: Application) {
    private val client = ApiClient()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var active: Job? = null
    val session = AuthSession(HttpAuthRepository(client), AndroidSessionStore(application))

    private val pendingAccounts = AndroidPendingAccountStore(application)
    fun newOnboarding() = OnboardingSession(AuthenticatedOnboardingBackend(session, client), pendingAccounts,
        { IdempotencyKey(UUID.randomUUID().toString()) }, ::profileToday)

    fun execute(action: suspend () -> Unit) { scope.launch { action() } }

    fun submit(action: suspend () -> Unit) {
        if (active?.isActive == true) return
        active = scope.launch { action() }
    }
}

/** API23-compatible calendar projection, never a silent unknown-zone-to-GMT fallback. */
private fun profileToday(zone: String): String? {
    if (zone !in TimeZone.getAvailableIDs()) return null
    val calendar = Calendar.getInstance(TimeZone.getTimeZone(zone), Locale.ROOT)
    return "%04d-%02d-%02d".format(Locale.ROOT, calendar.get(Calendar.YEAR),
        calendar.get(Calendar.MONTH) + 1, calendar.get(Calendar.DAY_OF_MONTH))
}
