package org.cuenora.shared.savings

import kotlinx.serialization.json.JsonPrimitive
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.onboarding.*
import org.cuenora.shared.repository.*

interface SavingsBackend {
    suspend fun profile(): RepositoryResult<OnboardingProfile>
    suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>>
    suspend fun terms(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery): RepositoryResult<Page<SavingsTermsVersion>>
    suspend fun rates(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery): RepositoryResult<Page<SavingsRateVersion>>
    suspend fun forecast(owner: OwnerId, account: AccountId, start: BusinessDate, end: BusinessDate): RepositoryResult<SavingsEstimate>
    suspend fun write(pending: PendingSavings): RepositoryResult<Unit>
}
/** One presentation lifetime, pinned to its first verified owner. No automatic write retries. */
class AuthenticatedSavingsBackend(private val session: AuthSession, private val api: ApiClient,
    private val active: () -> Boolean) : SavingsBackend {
    private var owner: OwnerId? = null
    override suspend fun profile(): RepositoryResult<OnboardingProfile> = session.authorized { token ->
        if (!active()) denied() else when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> if (!active() || owner != null && owner != result.value.ownerId) denied()
                else { owner = result.value.ownerId; result }
        }
    }
    private suspend fun <T> withOwner(expected: OwnerId, block: suspend (String) -> RepositoryResult<T>): RepositoryResult<T> = session.authorized { token ->
        if (!active()) denied() else when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> {
                val identity = result.value
                if (!active() || owner != expected || identity.ownerId != expected || !identity.emailVerified) denied()
                else block(token)
            }
        }
    }
    override suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>> {
        if (!active() || this.owner != owner) return denied()
        return org.cuenora.shared.recording.AuthenticatedRecordingBackend(session, api).accounts(owner)
    }
    override suspend fun terms(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery) = withOwner(owner) {
        HttpSavingsRepository(api, RepositoryContext(owner, it)).terms(account, query)
    }
    override suspend fun rates(owner: OwnerId, account: AccountId, query: SavingsHistoryQuery) = withOwner(owner) {
        HttpSavingsRepository(api, RepositoryContext(owner, it)).rates(account, query)
    }
    override suspend fun forecast(owner: OwnerId, account: AccountId, start: BusinessDate, end: BusinessDate) = withOwner(owner) {
        HttpSavingsRepository(api, RepositoryContext(owner, it)).forecast(account, start, end)
    }
    override suspend fun write(pending: PendingSavings): RepositoryResult<Unit> = withOwner(pending.owner) { token ->
        val repository = HttpSavingsRepository(api, RepositoryContext(pending.owner, token))
        val result = when (val a = pending.action) {
            is SavingsAction.Terms -> repository.appendTerms(a.account, a.command)
            is SavingsAction.Rate -> repository.appendRate(a.account, a.command)
            is SavingsAction.Receipt -> HttpTransactionRepository(api, RepositoryContext(pending.owner, token)).create(
                TransactionCreateCommand.InterestIncome(readMoney(JsonPrimitive(a.amount), a.currency), businessDate(a.occurredOn), a.account,
                    description = a.description), pending.key!!)
        }
        when (result) {
            is RepositoryResult.Success -> RepositoryResult.Success(Unit)
            is RepositoryResult.Failure -> result
        }
    }
    private fun denied() = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
}
