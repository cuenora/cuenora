package org.cuenora.shared.planning

import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.domain.BusinessDate
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*

/** A memory-only planning reader bound to a single signed-in UI lifetime. */
class AuthenticatedPlanningReader(private val session: AuthSession, private val api: ApiClient) : PlanningReader {
    private var owner: OwnerId? = null
    private var retired = false

    override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> = session.authorized { token ->
        if (retired) return@authorized RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
        val profile = HttpOnboardingRepository(api, token).profile()
        // A failed response may also arrive after disposal. Do not let a stale
        // presentation response expire the shared authenticated session.
        if (retired) return@authorized RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
        when (profile) {
            is RepositoryResult.Failure -> profile
            is RepositoryResult.Success -> {
                val identity = profile.value
                if (!identity.emailVerified || owner != null && owner != identity.ownerId) {
                    RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                } else {
                    owner = identity.ownerId
                    val result = HttpPlanningRepository(api, RepositoryContext(identity.ownerId, token)).load(through)
                    if (retired) RepositoryResult.Failure(ApiFailure.Http(403, null, null, null)) else result
                }
            }
        }
    }

    /** Retire this object with the signed-in screen lifetime; future requests cannot reuse it. */
    fun clear() { retired = true; owner = null }
}
