package org.cuenora.shared.savings

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import org.cuenora.shared.domain.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.validation.*

enum class SavingsStage { Closed, Loading, Browsing, Review, Uncertain, Saved, Failed, Blocked }
data class SavingsState(val stage: SavingsStage = SavingsStage.Closed, val account: Account? = null,
    val receivingAccounts: List<Account> = emptyList(), val terms: List<SavingsTermsVersion> = emptyList(), val rates: List<SavingsRateVersion> = emptyList(),
    val termsCursor: String? = null, val ratesCursor: String? = null, val estimate: SavingsEstimate? = null,
    val action: SavingsAction? = null, val busy: Boolean = false, val notice: String? = null,
    val zone: String? = null)

/** Serializes review/write/recovery. In-memory presentation retirement never deletes durable intent. */
class SavingsSession(private val backend: SavingsBackend, private val journal: SavingsJournal,
    private val origin: String, private val newKey: () -> IdempotencyKey, private val today: (String) -> String?) {
    private val mutable = MutableStateFlow(SavingsState())
    val state = mutable.asStateFlow()
    private val mutex = Mutex()
    private var retired = false
    private var owner: OwnerId? = null
    private var pending: PendingSavings? = null
    private fun currentDate(): BusinessDate? = state.value.zone?.let(today)?.let { runCatching { businessDate(it) }.getOrNull() }

    suspend fun open(account: Account) {
        if (retired || !mutex.tryLock()) return
        try {
            mutable.value = SavingsState(SavingsStage.Loading, account, busy = true)
            val result = backend.profile()
            if (retired) return
            val profile = (result as? RepositoryResult.Success)?.value
            if (profile == null || profile.ownerId != account.ownerId || owner != null && owner != profile.ownerId || account.type != AccountType.SAVINGS) {
                fail("Savings access could not be verified. Return and try again."); return
            }
            owner = profile.ownerId
            mutable.value = state.value.copy(zone = profile.timeZone)
            pending = journal.read(profile.ownerId, origin)
            if (retired) return
            pending?.let { original ->
                require(original.owner == profile.ownerId && original.origin == origin)
                mutable.value = state.value.copy(stage = SavingsStage.Uncertain, busy = false, action = original.action,
                    notice = "Resolve the original savings request. Nothing is resubmitted automatically.")
                return
            }
            if (!profile.emailVerified) {
                mutable.value = state.value.copy(stage = SavingsStage.Blocked, busy = false, notice = "Verify your email from Home first."); return
            }
            val accounts = backend.accounts(profile.ownerId)
            if (retired) return
            val eligible = (accounts as? RepositoryResult.Success)?.value.orEmpty()
            require(eligible.all { it.ownerId == profile.ownerId })
            mutable.value = state.value.copy(receivingAccounts = eligible.filter { it.status == AccountStatus.ACTIVE },
                notice = if (accounts is RepositoryResult.Failure) "Receiving accounts unavailable; refresh before recording interest." else null)
            loadHistory(account)
        } catch (cancelled: CancellationException) {
            if (!retired) fail("Loading interrupted. Retry explicitly.")
            throw cancelled
        } catch (_: Exception) { if (!retired) fail("Secure recovery is unavailable. Do not create a replacement request.") }
        finally { mutex.unlock() }
    }
    private suspend fun loadHistory(account: Account) {
        val terms = backend.terms(owner!!, account.id, SavingsHistoryQuery())
        if (retired) return
        val rates = backend.rates(owner!!, account.id, SavingsHistoryQuery())
        if (retired) return
        if (terms !is RepositoryResult.Success || rates !is RepositoryResult.Success) { fail("Savings history unavailable. Retry the read; no write was sent."); return }
        mutable.value = state.value.copy(stage = SavingsStage.Browsing, busy = false, terms = terms.value.items,
            rates = rates.value.items, termsCursor = terms.value.nextCursor, ratesCursor = rates.value.nextCursor,
            estimate = null, action = null, notice = null)
    }
    suspend fun moreTerms() = more(true)
    suspend fun moreRates() = more(false)
    private suspend fun more(terms: Boolean) {
        if (retired || state.value.stage != SavingsStage.Browsing || !mutex.tryLock()) return
        try {
            val s = state.value
            val cursor = (if (terms) s.termsCursor else s.ratesCursor) ?: return
            mutable.value = s.copy(busy = true, notice = null)
            if (terms) {
                when (val result = backend.terms(owner!!, s.account!!.id, SavingsHistoryQuery(after = businessDate(cursor)))) {
                    is RepositoryResult.Success -> if (!retired) {
                        require(result.value.items.all { it.terms.effectiveOn > businessDate(cursor) })
                        mutable.value = s.copy(terms = s.terms + result.value.items, termsCursor = result.value.nextCursor)
                    }
                    is RepositoryResult.Failure -> if (!retired) mutable.value = s.copy(notice = "More terms unavailable. Retry explicitly.")
                }
            } else {
                when (val result = backend.rates(owner!!, s.account!!.id, SavingsHistoryQuery(after = businessDate(cursor)))) {
                    is RepositoryResult.Success -> if (!retired) {
                        require(result.value.items.all { it.rate.effectiveOn > businessDate(cursor) })
                        mutable.value = s.copy(rates = s.rates + result.value.items, ratesCursor = result.value.nextCursor)
                    }
                    is RepositoryResult.Failure -> if (!retired) mutable.value = s.copy(notice = "More rates unavailable. Retry explicitly.")
                }
            }
        } catch (cancelled: CancellationException) { if (!retired) mutable.value = state.value.copy(busy = false); throw cancelled }
        catch (_: Exception) { if (!retired) mutable.value = state.value.copy(busy = false, notice = "History could not be read safely.") }
        finally { mutex.unlock() }
    }
    fun review(action: SavingsAction) {
        if (retired || state.value.busy || state.value.stage != SavingsStage.Browsing) return
        val account = state.value.account ?: return
        val receiver = state.value.receivingAccounts.firstOrNull { it.id == action.account }
        val valid = when (action) {
            is SavingsAction.Receipt -> receiver != null && receiver.status == AccountStatus.ACTIVE && action.currency == receiver.currency &&
                validateTransactionForm(action.form(), state.value.receivingAccounts).isLocallyValid &&
                runCatching { currentDate()?.let { businessDate(action.occurredOn) <= it } == true }.getOrDefault(false)
            else -> action.account == account.id
        }
        mutable.value = if (valid) state.value.copy(stage = SavingsStage.Review, action = action, notice = null)
            else state.value.copy(notice = "Check the date, positive amount, currency and receiving account. Actual receipts require a confirmed profile zone and cannot be future dated.")
    }
    fun edit() { if (!retired && !state.value.busy && state.value.stage == SavingsStage.Review) mutable.value = state.value.copy(stage = SavingsStage.Browsing, action = null) }
    suspend fun forecast(start: String, end: String) {
        if (retired || state.value.stage != SavingsStage.Browsing || !mutex.tryLock()) return
        try {
            mutable.value = state.value.copy(busy = true, estimate = null, notice = null)
            val a = runCatching { businessDate(start) }.getOrNull(); val b = runCatching { businessDate(end) }.getOrNull()
            if (a == null || b == null || a > b || currentDate() == null || a < currentDate()!!) {
                mutable.value = state.value.copy(busy = false, notice = "Choose explicit start/end dates, starting today or later in your confirmed profile zone (maximum 366 days)."); return
            }
            when (val result = backend.forecast(owner!!, state.value.account!!.id, a, b)) {
                is RepositoryResult.Success -> if (!retired) {
                    require(result.value.currency == state.value.account!!.currency && result.value.accountId == state.value.account!!.id && result.value.start == a && result.value.end == b)
                    mutable.value = state.value.copy(busy = false, estimate = result.value)
                }
                is RepositoryResult.Failure -> if (!retired) mutable.value = state.value.copy(busy = false,
                    notice = "Estimate unavailable. Check the 1–366 day range, effective terms/rate coverage and any policy changes within the range; then retry. No money was posted.")
            }
        } catch (cancelled: CancellationException) { if (!retired) mutable.value = state.value.copy(busy = false); throw cancelled }
        catch (_: Exception) { if (!retired) mutable.value = state.value.copy(busy = false, notice = "Estimate unavailable. No money was posted.") }
        finally { mutex.unlock() }
    }
    suspend fun confirm() {
        if (retired || state.value.stage !in setOf(SavingsStage.Review, SavingsStage.Uncertain) || !mutex.tryLock()) return
        try {
            val s = state.value
            val original = pending ?: PendingSavings(owner!!, origin, s.action!!,
                if (s.action is SavingsAction.Receipt) newKey() else null).also { pending = it }
            mutable.value = s.copy(busy = true, notice = null)
            journal.save(original.owner, origin, original)
            if (retired) return
            when (val result = backend.write(original)) {
                is RepositoryResult.Success -> {
                    if (retired) return
                    journal.clear(original.owner, origin)
                    if (retired) return
                    pending = null
                    mutable.value = s.copy(stage = SavingsStage.Saved, busy = false, notice = "The server confirmed the original request.")
                }
                is RepositoryResult.Failure -> if (!retired) {
                    // Immutable date slots cannot have accepted this command if the server
                    // reports conflicting content. A definite rejection is not an unknown receipt.
                    val status = (result.error as? ApiFailure.Http)?.status
                    if (original.action !is SavingsAction.Receipt && status in setOf(400, 409, 422)) {
                        journal.clear(original.owner, origin)
                        if (retired) return
                        pending = null
                        mutable.value = s.copy(stage = SavingsStage.Browsing, action = null, busy = false,
                            notice = "Configuration rejected. The existing date slot is unchanged. Refresh history and review a corrected version; nothing was posted.")
                    } else uncertain()
                }
            }
        } catch (cancelled: CancellationException) { if (!retired) uncertain(); throw cancelled }
        catch (_: Exception) { if (!retired) uncertain() }
        finally { mutex.unlock() }
    }
    private fun uncertain() { mutable.value = state.value.copy(stage = SavingsStage.Uncertain, busy = false,
        notice = "Completion is unconfirmed. Retry only the original request; conflicting configuration dates require resolution without replacing the stored intent.") }
    private fun fail(message: String) { mutable.value = state.value.copy(stage = SavingsStage.Failed, busy = false, notice = message) }
    fun close() { if (!retired && !state.value.busy) mutable.value = SavingsState() }
    fun clear() { retired = true; pending = null; owner = null; mutable.value = SavingsState() }
}

fun authenticatedSavingsSession(session: org.cuenora.shared.auth.AuthSession, api: org.cuenora.shared.network.ApiClient,
    journal: SavingsJournal, origin: String, newKey: () -> IdempotencyKey, today: (String) -> String?): SavingsSession {
    lateinit var flow: SavingsSession
    val backend = AuthenticatedSavingsBackend(session, api) { flow.state.value.stage != SavingsStage.Closed }
    flow = SavingsSession(backend, journal, origin, newKey, today)
    return flow
}
