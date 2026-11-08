package org.cuenora.shared.recording

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.serialization.json.JsonPrimitive
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.*

enum class RecordingStage { Closed, Loading, Form, Review, Uncertain, Saved, Failed, Blocked }
data class RecordingState(
    val stage: RecordingStage = RecordingStage.Closed,
    val profile: OnboardingProfile? = null,
    val form: TransactionForm = TransactionForm(TransactionKind.EXPENSE, "", Currency.SEK, ""),
    val accounts: List<Account> = emptyList(),
    val categories: List<CatalogCategory> = emptyList(),
    val issues: List<ValidationIssue> = emptyList(),
    val busy: Boolean = false,
    val notice: String? = null,
    val accountsUnavailable: Boolean = false,
    val categoriesUnavailable: Boolean = false,
)

/** One signed-in presentation lifetime. Drafts never enter saved instance state or storage. */
class RecordingSession(
    private val backend: RecordingBackend,
    private val store: PendingTransactionStore,
    private val categories: suspend (OwnerId) -> RepositoryResult<List<CatalogCategory>>,
    private val newKey: () -> IdempotencyKey,
    private val today: (String) -> String?,
) {
    private val mutable = MutableStateFlow(RecordingState())
    val state = mutable.asStateFlow()
    private val mutex = Mutex()
    private var retired = false
    private var pending: PendingTransaction? = null
    private var owner: OwnerId? = null
    private var requestedAccount: Account? = null

    suspend fun begin(account: Account? = null) {
        if (retired || !mutex.tryLock()) return
        try {
            requestedAccount = account
            mutable.value = RecordingState(stage = RecordingStage.Loading, busy = true)
            val profile = when (val result = backend.profile()) {
                is RepositoryResult.Success -> result.value
                is RepositoryResult.Failure -> { fail("Profile unavailable. Try again; no record has been assumed saved."); return }
            }
            if (retired) return
            if (owner != null && owner != profile.ownerId) { fail("Access unavailable. Sign in again."); return }
            owner = profile.ownerId
            mutable.value = state.value.copy(profile = profile)
            pending = store.read(profile.ownerId)
            if (retired) return
            pending?.let {
                require(it.owner == profile.ownerId)
                mutable.value = state.value.copy(stage = RecordingStage.Uncertain, busy = false,
                    form = it.command.toForm(), notice = "Resolve the original operation. Nothing is resubmitted automatically.")
                return
            }
            if (!profile.emailVerified || profile.timeZone == null || today(profile.timeZone) == null) {
                mutable.value = state.value.copy(stage = RecordingStage.Blocked, busy = false,
                    notice = "Verify your email and explicitly confirm your profile time zone from Home before recording activity.")
                return
            }
            val accountsResult = backend.accounts(profile.ownerId)
            if (retired) return
            val accounts = (accountsResult as? RepositoryResult.Success)?.value.orEmpty()
            if (accounts.any { it.ownerId != profile.ownerId } || account != null &&
                (account.ownerId != profile.ownerId || accounts.none { it.id == account.id })) {
                fail("The selected account could not be verified. Return and retry the account read."); return
            }
            val catalogResult = categories(profile.ownerId)
            if (retired) return
            mutable.value = state.value.copy(stage = RecordingStage.Form, busy = false, accounts = accounts,
                categories = (catalogResult as? RepositoryResult.Success)?.value.orEmpty(),
                accountsUnavailable = accountsResult is RepositoryResult.Failure,
                categoriesUnavailable = catalogResult is RepositoryResult.Failure,
                form = TransactionForm(TransactionKind.EXPENSE, "", account?.currency ?: Currency.SEK, "", account?.id))
        } catch (cancelled: CancellationException) {
            if (!retired) fail("Loading interrupted. Try again.")
            throw cancelled
        } catch (_: Exception) {
            if (!retired) fail("Secure recovery is unavailable. Do not create a replacement operation; try again.")
        } finally { mutex.unlock() }
    }

    suspend fun retryLoad() = begin(requestedAccount)

    suspend fun confirmZone(zone: String) {
        if (retired || state.value.stage != RecordingStage.Blocked || !mutex.tryLock()) return
        var reload = false
        try {
            val profile = state.value.profile ?: return
            if (!profile.emailVerified || today(zone) == null) {
                mutable.value = state.value.copy(notice = "Enter a supported IANA time zone and verify your email first.")
                return
            }
            mutable.value = state.value.copy(busy = true)
            when (val result = backend.confirmZone(profile.ownerId, zone)) {
                is RepositoryResult.Success -> { if (!retired) reload = true }
                is RepositoryResult.Failure -> if (!retired) mutable.value = state.value.copy(busy = false,
                    notice = "Time zone confirmation is unavailable. Check your profile and try again; no financial record was sent.")
            }
        } catch (cancelled: CancellationException) {
            if (!retired) mutable.value = state.value.copy(busy = false)
            throw cancelled
        } finally { mutex.unlock() }
        if (reload) begin(requestedAccount)
    }

    fun update(form: TransactionForm) {
        if (!retired && !state.value.busy && state.value.stage == RecordingStage.Form)
            mutable.value = state.value.copy(form = form, issues = emptyList(), notice = null)
    }
    fun review() {
        if (retired || state.value.busy || state.value.stage != RecordingStage.Form) return
        val s = state.value
        val issues = validateTransactionForm(s.form, s.accounts).issues.toMutableList()
        if (s.form.kind !in listOf(TransactionKind.INCOME, TransactionKind.EXPENSE)) issues += ValidationIssue("kind", "unsupported")
        if (s.form.accountId != null && s.accounts.none { it.id == s.form.accountId }) issues += ValidationIssue("accountId", "unavailable")
        if (s.form.categoryId != null && s.categories.none { it.id == s.form.categoryId && it.kind == s.form.kind })
            issues += ValidationIssue("categoryId", "incompatible")
        val date = runCatching { businessDate(s.form.occurredOn) }.getOrNull()
        val current = s.profile?.timeZone?.let(today)?.let { runCatching { businessDate(it) }.getOrNull() }
        if (date != null && (current == null || date > current)) issues += ValidationIssue("occurredOn", "future_date")
        mutable.value = s.copy(issues = issues, stage = if (issues.isEmpty()) RecordingStage.Review else RecordingStage.Form)
    }
    fun edit() {
        if (!retired && !state.value.busy && state.value.stage == RecordingStage.Review)
            mutable.value = state.value.copy(stage = RecordingStage.Form)
    }
    fun cancel() {
        if (!retired && !state.value.busy && state.value.stage in setOf(RecordingStage.Form, RecordingStage.Review, RecordingStage.Saved,
                RecordingStage.Failed, RecordingStage.Blocked, RecordingStage.Uncertain)) {
            mutable.value = RecordingState()
            // Pending journal remains; navigation never discards an uncertain write.
        }
    }
    fun clear() { retired = true; pending = null; mutable.value = RecordingState() }

    suspend fun save() {
        if (retired || state.value.stage !in setOf(RecordingStage.Review, RecordingStage.Uncertain) || !mutex.tryLock()) return
        var confirmed = false
        try {
            mutable.value = state.value.copy(busy = true, notice = null)
            val s = state.value
            val operation = pending ?: PendingTransaction(s.profile!!.ownerId, s.form.toCommand(), newKey()).also { pending = it }
            // Must be durable before any network write; repeats cannot replace the original.
            store.save(operation)
            if (retired) return
            when (val result = backend.create(operation)) {
                is RepositoryResult.Success -> {
                    confirmed = true
                    if (retired) return
                    store.clear(operation.owner)
                    if (retired) return
                    pending = null
                    mutable.value = s.copy(stage = RecordingStage.Saved, busy = false, notice = "Recorded successfully.")
                }
                is RepositoryResult.Failure -> {
                    if (retired) return
                    if ((result.error as? ApiFailure.Http)?.status in setOf(400, 422)) {
                        // An explicit validation rejection proves this attempt made no record.
                        store.clear(operation.owner)
                        if (retired) return
                        pending = null
                        mutable.value = s.copy(stage = RecordingStage.Form, busy = false,
                            notice = "The service rejected these details. Correct the form and review it again; nothing was recorded by this request.")
                    } else mutable.value = s.copy(stage = RecordingStage.Uncertain, busy = false,
                        notice = failureMessage(result.error))
                }
            }
        } catch (cancelled: CancellationException) {
            if (!retired) uncertain("Completion is unknown. Resolve the original request.")
            throw cancelled
        } catch (_: Exception) {
            if (!retired) uncertain(if (confirmed) "Saved on the server, but local acknowledgement failed. Resolve the original request safely."
                else "Secure storage or completion is unavailable. Resolve the original operation; do not create another.")
        } finally { mutex.unlock() }
    }
    private fun fail(message: String) {
        if (!retired) mutable.value = state.value.copy(stage = RecordingStage.Failed, busy = false, notice = message)
    }
    private fun uncertain(message: String) { mutable.value = state.value.copy(stage = RecordingStage.Uncertain, busy = false, notice = message) }
}

private fun TransactionForm.toCommand(): TransactionCreateCommand {
    val amount = readMoney(JsonPrimitive(amount), currency)
    val date = businessDate(occurredOn)
    return if (kind == TransactionKind.INCOME) TransactionCreateCommand.Income(amount, date, accountId, categoryId, description)
        else TransactionCreateCommand.Expense(amount, date, accountId, categoryId, description)
}
private fun TransactionCreateCommand.toForm(): TransactionForm = when (this) {
    is TransactionCreateCommand.Income -> TransactionForm(TransactionKind.INCOME, (writeMoney(amount) as JsonPrimitive).content,
        amount.currency, wireDate(occurredOn), accountId, categoryId, description)
    is TransactionCreateCommand.Expense -> TransactionForm(TransactionKind.EXPENSE, (writeMoney(amount) as JsonPrimitive).content,
        amount.currency, wireDate(occurredOn), accountId, categoryId, description)
    else -> error("Unsupported recording operation")
}
private fun failureMessage(error: ApiFailure): String = when (error) {
    is ApiFailure.Http -> when (error.status) {
        401 -> "Session unavailable. Sign in again; the original operation is preserved, not automatically resubmitted."
        403 -> "Access unavailable. Check email verification and profile time zone from Home, then resolve the original request."
        400, 422 -> "The service rejected the original details. No success is assumed; the original operation is retained for safe resolution."
        409 -> "The original operation could not be resolved. Do not create a replacement; retry later."
        else -> "Completion has not been confirmed. Retry only the original request."
    }
    else -> "Completion is unknown. Check your connection and retry only the original request."
}

/** Bind the optional global catalog read to the same current identity as the form. */
fun authenticatedRecordingSession(
    session: org.cuenora.shared.auth.AuthSession,
    api: org.cuenora.shared.network.ApiClient,
    store: PendingTransactionStore,
    newKey: () -> IdempotencyKey,
    today: (String) -> String?,
): RecordingSession = RecordingSession(AuthenticatedRecordingBackend(session, api), store, { owner ->
    session.authorized { token ->
        when (val profile = org.cuenora.shared.onboarding.HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> profile
            is RepositoryResult.Success -> if (profile.value.ownerId != owner)
                RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
            else HttpCategoryRepository(api, RepositoryContext(owner, token)).list()
        }
    }
}, newKey, today)
