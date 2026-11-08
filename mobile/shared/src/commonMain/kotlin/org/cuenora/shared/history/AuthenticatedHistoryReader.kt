package org.cuenora.shared.history

import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*

interface HistoryReader {
    suspend fun list(query: TransactionQuery): RepositoryResult<Page<TransactionRecord>>
    suspend fun accounts(query: AccountListQuery): RepositoryResult<Page<AccountWithBalance>>
    suspend fun categories(): RepositoryResult<List<CatalogCategory>>
}

/** Memory-only reader bound to one verified owner and signed-in presentation lifetime. */
class AuthenticatedHistoryReader(private val session: AuthSession, private val api: ApiClient) : HistoryReader {
    private var owner: OwnerId? = null

    suspend fun list(): RepositoryResult<Page<TransactionRecord>> = list(TransactionQuery())
    override suspend fun list(query: TransactionQuery) = authorized { HttpTransactionRepository(api, it).list(query) }
    override suspend fun accounts(query: AccountListQuery) = authorized { HttpAccountRepository(api, it).list(query) }
    override suspend fun categories() = authorized { HttpCategoryRepository(api, it).list() }

    private suspend fun <T> authorized(read: suspend (RepositoryContext) -> RepositoryResult<T>): RepositoryResult<T> =
        session.authorized { token ->
            when (val result = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> result
                is RepositoryResult.Success -> {
                    val profile = result.value
                    if (!profile.emailVerified || owner != null && owner != profile.ownerId) {
                        RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                    } else {
                        owner = profile.ownerId
                        read(RepositoryContext(profile.ownerId, token))
                    }
                }
            }
        }
}
