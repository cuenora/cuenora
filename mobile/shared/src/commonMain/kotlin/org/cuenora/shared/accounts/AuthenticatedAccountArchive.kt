package org.cuenora.shared.accounts

import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.Account
import org.cuenora.shared.domain.AccountStatus
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*

/** One authenticated mutation adapter per signed-in UI lifetime. */
class AuthenticatedAccountArchive(private val session: AuthSession, private val api: ApiClient) {
    private var owner: OwnerId? = null
    private var retired = false

    /** Prevents late UI work from continuing after the signed-in account subtree is disposed. */
    fun clear() { retired = true }

    suspend fun update(account: Account, status: AccountStatus): RepositoryResult<Account> =
        session.authorized { token ->
            if (retired) return@authorized denied()
            when (val profile = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> profile
                is RepositoryResult.Success -> {
                    val identity = profile.value
                    if (retired || !identity.emailVerified || account.ownerId != identity.ownerId ||
                        owner != null && owner != identity.ownerId) {
                        denied()
                    } else {
                        owner = identity.ownerId
                        val result = HttpAccountRepository(api, RepositoryContext(identity.ownerId, token))
                            .update(UpdateAccountCommand(account.id, status = PatchField.Value(status)))
                        // If the UI was retired during the request, do not publish its late result.
                        if (retired) denied() else result
                    }
                }
            }
        }

    private fun denied() = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
}
