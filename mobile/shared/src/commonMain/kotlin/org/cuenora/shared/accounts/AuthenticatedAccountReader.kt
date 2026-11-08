package org.cuenora.shared.accounts

import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*

interface AccountReader {
    suspend fun list(query: AccountListQuery = AccountListQuery()): RepositoryResult<Page<AccountWithBalance>>
}

/** One reader per signed-in UI lifetime; neither credentials nor financial records are cached. */
class AuthenticatedAccountReader(private val session: AuthSession, private val api: ApiClient) : AccountReader {
    // Accessed only inside the session's ordering domain. A different owner needs a new reader.
    private var owner: OwnerId? = null

    override suspend fun list(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>> =
        session.authorized { token ->
            when (val profile = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> profile
                is RepositoryResult.Success -> {
                    val identity = profile.value
                    if (!identity.emailVerified || (owner != null && owner != identity.ownerId)) {
                        RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                    } else {
                        owner = identity.ownerId
                        HttpAccountRepository(api, RepositoryContext(identity.ownerId, token)).list(query)
                    }
                }
            }
        }
}
