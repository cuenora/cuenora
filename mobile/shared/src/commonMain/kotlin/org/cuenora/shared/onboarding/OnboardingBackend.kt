package org.cuenora.shared.onboarding

import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*

interface OnboardingBackend {
    suspend fun profile(): RepositoryResult<OnboardingProfile>
    suspend fun requestVerification(): RepositoryResult<Unit>
    suspend fun confirmVerification(token: String): RepositoryResult<Unit>
    // Zone confirmation is bound to the profile the user reviewed.
    suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String>
    suspend fun hasAccounts(owner: OwnerId): RepositoryResult<Boolean>
    suspend fun create(pending: PendingAccount): RepositoryResult<Unit>
}

class AuthenticatedOnboardingBackend(private val session: AuthSession, private val api: ApiClient) : OnboardingBackend {
    override suspend fun profile() = session.authorized { HttpOnboardingRepository(api, it).profile() }
    override suspend fun requestVerification() = session.authorized { HttpOnboardingRepository(api, it).requestVerification() }
    override suspend fun confirmVerification(token: String) = session.authorized { HttpOnboardingRepository(api, it).confirmVerification(token) }
    override suspend fun confirmZone(owner: OwnerId, zone: String) = withOwner(owner) { token ->
        HttpOnboardingRepository(api, token).confirmZone(zone)
    }
    override suspend fun hasAccounts(owner: OwnerId) = withOwner(owner) { token ->
        when (val result = HttpAccountRepository(api, RepositoryContext(owner, token)).list(AccountListQuery(limit = 1))) {
            is RepositoryResult.Success -> RepositoryResult.Success(result.value.items.isNotEmpty())
            is RepositoryResult.Failure -> result
        }
    }
    override suspend fun create(pending: PendingAccount) = withOwner(pending.owner) { token ->
        when (val result = HttpAccountRepository(api, RepositoryContext(pending.owner, token)).create(pending.command, pending.key)) {
            is RepositoryResult.Success -> RepositoryResult.Success(Unit)
            is RepositoryResult.Failure -> result
        }
    }
    private suspend fun <T> withOwner(owner: OwnerId, block: suspend (String) -> RepositoryResult<T>): RepositoryResult<T> =
        session.authorized { token ->
            when (val profile = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> profile
                is RepositoryResult.Success -> if (profile.value.ownerId != owner || !profile.value.emailVerified)
                    RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                else block(token)
            }
        }
}
