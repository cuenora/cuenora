package org.cuenora.shared.onboarding

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.*
import org.cuenora.shared.serialization.MoneyAmountSerializer
import org.cuenora.shared.validation.*

enum class OnboardingStage { Loading, Verify, Empty, Existing, Form, Review, Uncertain, Complete, Failed }
enum class OnboardingNotice { MailAccepted, InvalidChallenge, InvalidForm, ZoneRequired, NetworkUnavailable,
    AccessDenied, RateLimited, ServiceUnavailable, StorageUnavailable, WriteUncertain, Saved }
data class OnboardingState(
    val stage: OnboardingStage = OnboardingStage.Loading,
    val busy: Boolean = false,
    val profile: OnboardingProfile? = null,
    val form: AccountForm? = null,
    val notice: OnboardingNotice? = null,
    val issues: List<ValidationIssue> = emptyList(),
)

/** One signed-in account writer; the encrypted journal survives process loss. */
class OnboardingSession(
    private val backend: OnboardingBackend,
    private val store: PendingAccountStore,
    private val newKey: () -> IdempotencyKey,
    private val today: (String) -> String?,
) {
    private val mutex = Mutex()
    private val mutableState = MutableStateFlow(OnboardingState())
    val state: StateFlow<OnboardingState> = mutableState.asStateFlow()
    private var pending: PendingAccount? = null
    private var retired = false
    private var additionalIntent = false
    private var returnStage = OnboardingStage.Empty

    /** Explicit creation intent, independent of whether the owner already has accounts. */
    suspend fun beginAdditional() = operation {
        additionalIntent = true
        loadProfile()
    }
    fun leaveAdditional() {
        if (retired || state.value.busy) return
        cancel()
        additionalIntent = false
    }
    fun clear() {
        retired = true
        additionalIntent = false
        pending = null
        mutableState.value = OnboardingState()
    }
    private suspend fun <T> active(block: suspend () -> T): T {
        if (retired) throw CancellationException("Account creation retired")
        val result = block()
        if (retired) throw CancellationException("Account creation retired")
        return result
    }

    suspend fun load() = operation {
        loadProfile()
    }
    private suspend fun loadProfile() {
        when (val result = active { backend.profile() }) {
            is RepositoryResult.Failure -> fail(result.error, OnboardingStage.Failed)
            is RepositoryResult.Success -> {
                val profile = result.value
                mutableState.value = OnboardingState(profile = profile, busy = true)
                pending = null
                if (!profile.emailVerified) {
                    mutableState.value = state.value.copy(stage = OnboardingStage.Verify)
                } else {
                    pending = active { store.read(profile.ownerId) }
                    if (pending != null) {
                        require(pending!!.owner == profile.ownerId)
                        mutableState.value = state.value.copy(stage = OnboardingStage.Uncertain, notice = OnboardingNotice.WriteUncertain)
                    } else when (val accounts = active { backend.hasAccounts(profile.ownerId) }) {
                        is RepositoryResult.Failure -> fail(accounts.error, OnboardingStage.Failed)
                        is RepositoryResult.Success -> {
                            returnStage = if (accounts.value) OnboardingStage.Existing else OnboardingStage.Empty
                            mutableState.value = state.value.copy(stage = returnStage)
                            if (additionalIntent) enterForm()
                        }
                    }
                }
            }
        }
    }
    suspend fun requestVerification() = operation {
        if (state.value.stage != OnboardingStage.Verify) return@operation
        when (val result = active { backend.requestVerification() }) {
            is RepositoryResult.Success -> mutableState.value = state.value.copy(notice = OnboardingNotice.MailAccepted)
            is RepositoryResult.Failure -> fail(result.error)
        }
    }
    suspend fun confirmVerification(token: String) = operation {
        if (state.value.stage != OnboardingStage.Verify) return@operation
        if (!Regex("ev1_[0-9a-f]{64}").matches(token)) {
            mutableState.value = state.value.copy(notice = OnboardingNotice.InvalidChallenge)
            return@operation
        }
        when (val result = active { backend.confirmVerification(token) }) {
            is RepositoryResult.Success -> loadProfile()
            is RepositoryResult.Failure -> fail(result.error)
        }
    }
    fun begin() {
        if (retired || state.value.busy || state.value.stage != OnboardingStage.Empty) return
        if (state.value.profile == null) return
        enterForm()
    }
    private fun enterForm() {
        mutableState.value = state.value.copy(stage = OnboardingStage.Form, notice = null,
            form = AccountForm("", AccountType.NORMAL, Currency.SEK, null, null))
    }
    fun edit() {
        if (!retired && !state.value.busy && state.value.stage == OnboardingStage.Review)
            mutableState.value = state.value.copy(stage = OnboardingStage.Form, notice = null)
    }
    fun cancel() {
        if (!retired && !state.value.busy && state.value.stage in setOf(OnboardingStage.Form, OnboardingStage.Review))
            mutableState.value = state.value.copy(stage = returnStage, form = null, notice = null, issues = emptyList())
        if (!retired && !state.value.busy) additionalIntent = false
    }
    fun updateForm(form: AccountForm) {
        if (!retired && !state.value.busy && state.value.stage == OnboardingStage.Form)
            mutableState.value = state.value.copy(form = form, issues = emptyList(), notice = null)
    }
    suspend fun confirmZone(zone: String) = operation {
        if (state.value.stage != OnboardingStage.Form) return@operation
        val profile = state.value.profile ?: return@operation
        when (val result = active { backend.confirmZone(profile.ownerId, zone) }) {
            is RepositoryResult.Failure -> fail(result.error)
            is RepositoryResult.Success -> {
                val form = state.value.form
                mutableState.value = state.value.copy(profile = profile.copy(timeZone = result.value), notice = null,
                    form = if (form?.openingBalanceAmount != null && form.openingBalanceOccurredOn.isNullOrEmpty())
                        form.copy(openingBalanceOccurredOn = today(result.value).orEmpty()) else form)
            }
        }
    }
    fun review(form: AccountForm) {
        if (retired || state.value.busy || state.value.stage != OnboardingStage.Form) return
        val validation = validateAccountForm(form)
        val zone = state.value.profile?.timeZone
        if (!validation.isLocallyValid) {
            mutableState.value = state.value.copy(form = form, issues = validation.issues, notice = OnboardingNotice.InvalidForm)
        } else if (form.openingBalanceAmount != null && zone == null) {
            mutableState.value = state.value.copy(form = form, notice = OnboardingNotice.ZoneRequired)
        } else if (form.openingBalanceOccurredOn != null && zone != null && today(zone)?.let {
                businessDate(form.openingBalanceOccurredOn) > businessDate(it)
            } == true) {
            mutableState.value = state.value.copy(form = form, notice = OnboardingNotice.InvalidForm)
        } else mutableState.value = state.value.copy(stage = OnboardingStage.Review, form = form, notice = null, issues = emptyList())
    }
    suspend fun save() = operation {
        if (state.value.stage !in setOf(OnboardingStage.Review, OnboardingStage.Uncertain)) return@operation
        val profile = state.value.profile ?: return@operation
        if (pending == null) {
            val form = state.value.form ?: return@operation
            val amount = form.openingBalanceAmount
            val opening = amount?.let { OpeningBalance(
                Json.decodeFromJsonElement(MoneyAmountSerializer(form.currency), JsonPrimitive(it)),
                businessDate(requireNotNull(form.openingBalanceOccurredOn))) }
            val record = PendingAccount(profile.ownerId, CreateAccountCommand(AccountName(form.name), form.type, form.currency, opening), newKey())
            // Persist first; an ambiguous persistence failure must never send a request.
            active { store.save(record) }
            pending = record
        }
        val record = pending ?: return@operation
        mutableState.value = state.value.copy(stage = OnboardingStage.Uncertain, notice = OnboardingNotice.WriteUncertain)
        when (val result = active { backend.create(record) }) {
            is RepositoryResult.Success -> {
                active { store.clear(record.owner) }
                pending = null
                mutableState.value = state.value.copy(stage = OnboardingStage.Complete, form = null, notice = OnboardingNotice.Saved)
            }
            is RepositoryResult.Failure -> {
                // Only explicit validation rejection proves no write occurred; uncertain outcomes stay locked.
                if ((result.error as? ApiFailure.Http)?.status == 400 && state.value.form != null) {
                    active { store.clear(record.owner) }; pending = null
                    fail(result.error, OnboardingStage.Form)
                } else fail(result.error, OnboardingStage.Uncertain)
            }
        }
    }
    private fun fail(error: ApiFailure, stage: OnboardingStage = state.value.stage) {
        if (retired) return
        val notice = when (error) {
            ApiFailure.Network, ApiFailure.Timeout -> OnboardingNotice.NetworkUnavailable
            is ApiFailure.Http -> when (error.status) {
                400 -> if (state.value.stage == OnboardingStage.Verify) OnboardingNotice.InvalidChallenge else OnboardingNotice.InvalidForm
                401, 403 -> OnboardingNotice.AccessDenied
                429 -> OnboardingNotice.RateLimited
                else -> OnboardingNotice.ServiceUnavailable
            }
            is ApiFailure.InvalidResponse -> OnboardingNotice.ServiceUnavailable
        }
        mutableState.value = state.value.copy(stage = stage, notice = notice)
    }
    private suspend fun operation(block: suspend () -> Unit) {
        if (retired || !mutex.tryLock()) return
        mutableState.value = state.value.copy(busy = true, notice = null)
        try { block() }
        catch (failure: PendingAccountStoreException) {
            if (!retired) mutableState.value = state.value.copy(stage = OnboardingStage.Failed, notice = OnboardingNotice.StorageUnavailable)
        } catch (cancelled: CancellationException) {
            if (!retired) mutableState.value = state.value.copy(stage = if (pending != null) OnboardingStage.Uncertain else OnboardingStage.Failed,
                notice = if (pending != null) OnboardingNotice.WriteUncertain else OnboardingNotice.ServiceUnavailable)
            throw cancelled
        } finally {
            if (!retired) mutableState.value = state.value.copy(busy = false)
            mutex.unlock()
        }
    }
}
