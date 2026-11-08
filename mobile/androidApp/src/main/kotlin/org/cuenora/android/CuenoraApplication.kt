package org.cuenora.android

import android.app.Application
import kotlinx.coroutines.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.auth.HttpAuthRepository
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.session.AndroidSessionStore

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

    fun submit(action: suspend () -> Unit) {
        if (active?.isActive == true) return
        active = scope.launch { action() }
    }
}
