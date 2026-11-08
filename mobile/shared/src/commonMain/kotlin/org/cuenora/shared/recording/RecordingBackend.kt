package org.cuenora.shared.recording

import kotlinx.serialization.json.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*

interface RecordingBackend {
    suspend fun profile(): RepositoryResult<OnboardingProfile>
    suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String>
    suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>>
    suspend fun create(pending: PendingTransaction): RepositoryResult<Unit>
}

/** Authenticated transaction entry point; each operation re-reads the server identity. */
class AuthenticatedRecordingBackend(private val session: AuthSession, private val api: ApiClient) : RecordingBackend {
    private var observedOwner: OwnerId? = null

    override suspend fun profile(): RepositoryResult<OnboardingProfile> = session.authorized { token ->
        when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> if (!rememberOwner(result.value.ownerId)) denied()
                else RepositoryResult.Success(result.value)
        }
    }

    override suspend fun confirmZone(owner: OwnerId, zone: String): RepositoryResult<String> = session.authorized { token ->
        when (val result = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> if (!rememberOwner(result.value.ownerId) || result.value.ownerId != owner || !result.value.emailVerified)
                denied()
            else HttpOnboardingRepository(api, token).confirmZone(zone)
        }
    }

    override suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>> = withOwner(owner) { token ->
        readAllAccounts(owner, token)
    }

    override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> = withOwner(pending.owner) { token ->
        // A journaled write may already have committed. Always let the server's idempotency
        // lookup decide its outcome; today's account state must not block a same-key replay.
        when (val result = HttpTransactionRepository(api, RepositoryContext(pending.owner, token)).create(pending.command, pending.key)) {
            is RepositoryResult.Failure -> result
            is RepositoryResult.Success -> RepositoryResult.Success(Unit)
        }
    }

    private suspend fun <T> withOwner(owner: OwnerId, block: suspend (String) -> RepositoryResult<T>): RepositoryResult<T> =
        session.authorized { token ->
            when (val result = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> result
                is RepositoryResult.Success -> {
                    val identity = result.value
                    if (!rememberOwner(identity.ownerId) || identity.ownerId != owner || !identity.emailVerified ||
                        identity.timeZone.isNullOrBlank()) denied()
                    else block(token)
                }
            }
        }

    private fun rememberOwner(owner: OwnerId): Boolean {
        val previous = observedOwner
        if (previous != null && previous != owner) return false
        observedOwner = owner
        return true
    }

    private suspend fun readAllAccounts(owner: OwnerId, token: String): RepositoryResult<List<Account>> {
        val repository = HttpAccountRepository(api, RepositoryContext(owner, token))
        val accounts = mutableListOf<Account>()
        val seen = mutableSetOf<String>()
        var cursor: String? = null
        var pageCount = 0
        while (true) {
            if (++pageCount > MAX_ACCOUNT_PAGES) return invalid(200)
            val page = when (val result = repository.list(AccountListQuery(limit = 100, cursor = cursor))) {
                is RepositoryResult.Failure -> return result
                is RepositoryResult.Success -> result.value
            }
            val pageIds = page.items.map { it.account.id }
            if (page.items.any { it.account.ownerId != owner } || pageIds.toSet().size != pageIds.size ||
                page.items.any { candidate -> accounts.any { it.id == candidate.account.id } }) return invalid(200)
            accounts.addAll(page.items.map { it.account })
            val next = page.nextCursor ?: return RepositoryResult.Success(accounts.toList())
            if (next.isBlank() || next.length > 2048 || !seen.add(next) || next == cursor) return invalid(200)
            cursor = next
        }
    }

    private fun denied() = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
    private fun invalid(status: Int) = RepositoryResult.Failure(ApiFailure.InvalidResponse(status, null))

    private companion object { const val MAX_ACCOUNT_PAGES = 1000 }
}
