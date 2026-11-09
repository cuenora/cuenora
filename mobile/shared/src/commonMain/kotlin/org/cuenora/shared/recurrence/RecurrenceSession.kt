package org.cuenora.shared.recurrence

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.*

/** Safe, credential-free presentation state for the recurrence screen. */
data class RecurrenceScreenState(
    val rules: List<RecurrenceRule> = emptyList(),
    val occurrences: List<RecurrenceOccurrence> = emptyList(),
    val accounts: List<AccountWithBalance> = emptyList(),
    val categories: List<CatalogCategory> = emptyList(),
    val profile: RecurrenceProfile? = null,
    val ruleFilter: RuleState? = null,
    val occurrenceFilter: OccurrenceState? = OccurrenceState.PLANNED,
    val rulesCursor: String? = null,
    val occurrencesCursor: String? = null,
    val hasMoreRules: Boolean = false,
    val hasMoreOccurrences: Boolean = false,
    val busy: Boolean = false,
    val notice: RecurrenceNotice? = null,
    val issues: List<ValidationIssue> = emptyList(),
    val review: RecurrenceReview? = null,
    val pendingRetry: Boolean = false,
    val pendingReview: RecurrenceReview? = null,
    val requiresRefresh: Boolean = false,
    val writeSucceeded: Boolean = false,
)
data class RecurrenceProfile(val emailVerified: Boolean, val timeZone: String?)
enum class RecurrenceNotice { AccessDenied, NetworkUnavailable, ServiceUnavailable, InvalidForm, ZoneRequired, WriteUncertain, RequestRejected, RefreshRequired, StorageUnavailable }

/**
 * Serializes every profile/resource operation through AuthSession's lock. One instance is
 * bound to one authenticated presentation lifetime; clear() retires it permanently.
 */
class AuthenticatedRecurrenceSession(
    private val session: AuthSession,
    private val api: ApiClient,
    private val journal: RecurrenceJournal,
    private val origin: String,
    private val newKey: () -> IdempotencyKey,
    private val today: (zone: String) -> String?,
) {
    private val mutableState = MutableStateFlow(RecurrenceScreenState())
    val state: StateFlow<RecurrenceScreenState> = mutableState.asStateFlow()
    private val mutex = Mutex()
    private val seenRuleCursors = mutableSetOf<String>()
    private val seenOccurrenceCursors = mutableSetOf<String>()
    private var owner: OwnerId? = null
    private var pending: PendingRecurrence? = null
    private var retired = false
    private var generation = 0

    suspend fun load() = operation {
        withOwner { currentOwner, context ->
            if (owner != null && owner != currentOwner) { denied(); return@withOwner }
            if (owner == null) {
                owner = currentOwner
            }
            pending = journal.read(currentOwner, origin)
            update { copy(pendingRetry = pending != null, pendingReview = pending?.let(::readPendingReview)) }
            loadResources(context, false)
        }
    }

    fun setRuleFilter(filter: RuleState?) {
        if (!retired && !state.value.busy) {
            seenRuleCursors.clear()
            update { copy(ruleFilter = filter, rulesCursor = null, hasMoreRules = false) }
        }
    }
    fun setOccurrenceFilter(filter: OccurrenceState?) {
        if (!retired && !state.value.busy) {
            seenOccurrenceCursors.clear()
            update { copy(occurrenceFilter = filter, occurrencesCursor = null, hasMoreOccurrences = false) }
        }
    }

    suspend fun refresh() = operation {
        withOwner { currentOwner, context ->
            if (owner != null && owner != currentOwner) { denied(); return@withOwner }
            if (owner == null) {
                owner = currentOwner
            }
            pending = journal.read(currentOwner, origin)
            update { copy(pendingRetry = pending != null, pendingReview = pending?.let(::readPendingReview)) }
            seenRuleCursors.clear()
            seenOccurrenceCursors.clear()
            loadResources(context, true)
        }
    }

    suspend fun loadMoreRules() = operation {
        withOwner { currentOwner, context ->
            if (currentOwner != owner) { denied(); return@withOwner }
            loadRules(context, true)
        }
    }
    suspend fun loadMoreOccurrences() = operation {
        withOwner { currentOwner, context ->
            if (currentOwner != owner) { denied(); return@withOwner }
            loadOccurrences(context, true)
        }
    }

    fun prepareCreate(draft: RecurrenceRuleDraft) = review { validateRule(draft, null) }
    fun prepareEdit(id: RuleId, draft: RecurrenceRuleDraft) = review {
        val current = state.value.rules.firstOrNull { it.id == id } ?: return@review invalid()
        validateRule(draft, current)
    }
    fun preparePause(id: RuleId) = review { if (state.value.rules.any { it.id == id }) update { copy(review = RecurrenceReview.PauseRule(id), notice = null) } else invalid() }
    fun prepareResume(id: RuleId) = review { if (state.value.rules.any { it.id == id }) update { copy(review = RecurrenceReview.ResumeRule(id), notice = null) } else invalid() }
    suspend fun prepareConfirm(id: OccurrenceId, actual: ActualPaymentDraft) = operation {
        if (pending == null && !state.value.requiresRefresh) validateActual(id, actual)
    }
    fun prepareSkip(id: OccurrenceId) = review { if (planned(id) != null) update { copy(review = RecurrenceReview.Skip(id), notice = null) } else invalid() }
    fun preparePostpone(id: OccurrenceId, dueOn: String) = review {
        val occurrence = planned(id) ?: return@review invalid()
        val parsed = runCatching { businessDate(dueOn) }.getOrNull()
        if (parsed == null || parsed <= occurrence.dueOn) invalid()
        else update { copy(review = RecurrenceReview.Postpone(id, dueOn), notice = null) }
    }

    fun cancelReview() { if (!retired && !state.value.busy) update { copy(review = null, issues = emptyList(), notice = null) } }

    suspend fun submitReviewed() = operation {
        if (pending != null || state.value.pendingRetry) { update { copy(notice = RecurrenceNotice.WriteUncertain) }; return@operation }
        if (state.value.requiresRefresh) { update { copy(notice = RecurrenceNotice.RefreshRequired) }; return@operation }
        val action = state.value.review ?: return@operation
        withOwner { currentOwner, context ->
            if (owner != currentOwner || !context.profile.emailVerified) { denied(); return@withOwner }
            val stored = journal.read(currentOwner, origin)
            if (!active() || owner != currentOwner) return@withOwner
            if (stored != null) {
                pending = stored
                val recovered = readPendingReview(stored)
                update { copy(pendingRetry = true, pendingReview = recovered, notice = RecurrenceNotice.WriteUncertain) }
                return@withOwner
            }
            val repo = HttpRecurrenceRepository(api, RepositoryContext(currentOwner, context.token))
            when (action) {
                is RecurrenceReview.CreateRule -> keyed(currentOwner, context, "/v1/recurring-rules", "POST", ruleBody(action.terms))
                is RecurrenceReview.EditRule -> nonKeyed(context) { repo.edit(action.id, action.terms, state.value.rules.first { it.id == action.id }) }
                is RecurrenceReview.PauseRule -> nonKeyed(context) { repo.pause(action.id) }
                is RecurrenceReview.ResumeRule -> nonKeyed(context) { repo.resume(action.id) }
                is RecurrenceReview.Confirm -> keyed(currentOwner, context, "/v1/recurring-occurrences/${wireId(action.id)}/confirm", "POST", actualBody(action.actual))
                is RecurrenceReview.Skip -> keyed(currentOwner, context, "/v1/recurring-occurrences/${wireId(action.id)}/skip", "POST", "{}")
                is RecurrenceReview.Postpone -> keyed(currentOwner, context, "/v1/recurring-occurrences/${wireId(action.id)}/postpone", "POST",
                    "{\"due_on\":${kotlinx.serialization.json.JsonPrimitive(action.dueOn)}}")
            }
        }
    }

    /** The only replay path, always the original stored URL, body and key. */
    suspend fun retryPending() = operation {
        withOwner { currentOwner, context ->
            val saved = pending ?: journal.read(currentOwner, origin) ?: return@withOwner
            if (!active() || owner != currentOwner) return@withOwner
            pending = saved
            val recovered = readPendingReview(saved)
            if (recovered == null) {
                update { copy(pendingRetry = true, pendingReview = null, notice = RecurrenceNotice.WriteUncertain) }
                return@withOwner
            }
            update { copy(pendingRetry = true, pendingReview = recovered) }
            if (currentOwner != saved.owner || currentOwner != owner || saved.origin != origin || !context.profile.emailVerified) { denied(); return@withOwner }
            replay(saved, context, priorUncertain = true)
        }
    }

    fun clear() {
        retired = true
        generation++
        mutableState.value = RecurrenceScreenState()
        pending = null
        owner = null
    }

    private suspend fun loadResources(context: BoundContext, preserve: Boolean) {
        val profile = context.profile
        when (val accountResult = readAllAccounts(owner!!, context.token)) {
            is RepositoryResult.Failure -> fail(accountResult.error, preserve)
            is RepositoryResult.Success -> {
                if (!active()) return
                when (val categoryResult = HttpCategoryRepository(api, RepositoryContext(owner!!, context.token)).list()) {
                is RepositoryResult.Failure -> fail(categoryResult.error, preserve)
                is RepositoryResult.Success -> {
                    if (!active()) return
                    update { copy(profile = RecurrenceProfile(profile.emailVerified, profile.timeZone), accounts = accountResult.value, categories = categoryResult.value, requiresRefresh = false, notice = null) }
                    val rulesLoaded = loadRules(context, false)
                    val ruleFailure = if (rulesLoaded) null else state.value.notice
                    loadOccurrences(context, false)
                    if (ruleFailure != null) update { copy(notice = ruleFailure) }
                }
            }
            }
        }
    }
    private suspend fun readAllAccounts(owner: OwnerId, token: String): RepositoryResult<List<AccountWithBalance>> {
        val repository = HttpAccountRepository(api, RepositoryContext(owner, token))
        val values = mutableListOf<AccountWithBalance>()
        val identities = mutableSetOf<AccountId>()
        val cursors = mutableSetOf<String>()
        var cursor: String? = null
        repeat(1000) {
            when (val result = repository.list(AccountListQuery(limit = 100, cursor = cursor))) {
                is RepositoryResult.Failure -> return result
                is RepositoryResult.Success -> {
                    if (result.value.items.any { !identities.add(it.account.id) }) {
                        return RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null))
                    }
                    values += result.value.items
                    val next = result.value.nextCursor ?: return RepositoryResult.Success(values)
                    if (next == cursor || !cursors.add(next)) return RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null))
                    cursor = next
                }
            }
        }
        return RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null))
    }
    private suspend fun loadRules(context: BoundContext, append: Boolean): Boolean {
        val current = state.value
        val cursor = if (append) current.rulesCursor ?: return true else null
        val result = HttpRecurrenceRepository(api, RepositoryContext(owner!!, context.token)).rules(cursor = cursor, state = current.ruleFilter)
        if (!active()) return false
        return when (result) {
            is RepositoryResult.Failure -> { fail(result.error, append); false }
            is RepositoryResult.Success -> {
                val page = result.value
                if (page.nextCursor != null && (page.nextCursor == cursor || page.nextCursor in seenRuleCursors)) { fail(ApiFailure.InvalidResponse(200, null), append); return false }
                if (cursor != null) seenRuleCursors += cursor
                update { copy(rules = (if (append) rules else emptyList()).plus(page.items).distinctBy { it.id }, rulesCursor = page.nextCursor, hasMoreRules = page.nextCursor != null, notice = null) }
                true
            }
        }
    }
    private suspend fun loadOccurrences(context: BoundContext, append: Boolean): Boolean {
        val current = state.value
        val cursor = if (append) current.occurrencesCursor ?: return true else null
        val result = HttpRecurrenceRepository(api, RepositoryContext(owner!!, context.token)).occurrences(cursor = cursor, state = current.occurrenceFilter)
        if (!active()) return false
        return when (result) {
            is RepositoryResult.Failure -> { fail(result.error, append); false }
            is RepositoryResult.Success -> {
                val page = result.value
                if (page.nextCursor != null && (page.nextCursor == cursor || page.nextCursor in seenOccurrenceCursors)) { fail(ApiFailure.InvalidResponse(200, null), append); return false }
                if (cursor != null) seenOccurrenceCursors += cursor
                update { copy(occurrences = (if (append) occurrences else emptyList()).plus(page.items).distinctBy { it.id }, occurrencesCursor = page.nextCursor, hasMoreOccurrences = page.nextCursor != null, notice = null) }
                true
            }
        }
    }

    private suspend fun keyed(owner: OwnerId, context: BoundContext, path: String, method: String, body: String) {
        if (!active() || this.owner != owner) return
        val key = newKey()
        val command = buildJsonObject { put("path", path); put("method", method); put("body", body) }.toString()
        val saved = PendingRecurrence(owner, origin, key, command)
        journal.save(owner, origin, saved)
        // Storage can suspend while the authenticated presentation is disposed.
        // Preserve its durable command, but leave sending to an explicit fresh-session retry.
        if (!active() || this.owner != owner) return
        pending = saved
        update { copy(pendingRetry = true, pendingReview = state.value.review) }
        replay(saved, context, priorUncertain = false)
    }

    private suspend fun replay(saved: PendingRecurrence, context: BoundContext, priorUncertain: Boolean) {
        if (!active() || owner != saved.owner) return
        val root = Json.parseToJsonElement(saved.command).jsonObject
        val path = root.string("path")
        require(path.startsWith("/v1/recurring-"))
        val method = HttpMethod.parse(root.string("method"))
        val body = Json.parseToJsonElement(root.string("body"))
        val result = api.request(path, method, body, RepositoryContext(saved.owner, context.token).headers(saved.key))
        when (result) {
            is ApiResult.Success -> {
                val repository = HttpRecurrenceRepository(api, RepositoryContext(saved.owner, context.token))
                val expectedStatus = if (path == "/v1/recurring-rules" || path.endsWith("/confirm")) 201 else 200
                val valid = result.status == expectedStatus && result.body != null && runCatching {
                    when {
                        path == "/v1/recurring-rules" -> repository.decodeCreatedRule(result.body)
                        path.endsWith("/confirm") -> repository.decodeConfirmedExpense(result.body, actualDraft(body.jsonObject))
                        path.endsWith("/skip") || path.endsWith("/postpone") -> {
                            val action = repository.decodeActionResult(result.body)
                            val expectedId = occurrenceId(path.substringAfter("/recurring-occurrences/").substringBefore('/'))
                            require(action.id == expectedId)
                            require(action.state == if (path.endsWith("/skip")) OccurrenceState.SKIPPED else OccurrenceState.PLANNED)
                            if (path.endsWith("/postpone")) require(action.dueOn == businessDate(body.jsonObject.string("due_on")))
                        }
                        else -> error("Unexpected recurrence command")
                    }
                }.isSuccess
                if (!valid) { update { copy(notice = RecurrenceNotice.WriteUncertain) }; return }
                journal.clear(saved.owner, origin)
                pending = null
                if (!active()) return
                update { copy(pendingRetry = false, pendingReview = null, review = null, writeSucceeded = true, notice = null) }
                loadResources(context, true)
            }
            is ApiResult.Failure -> {
                val status = (result.error as? ApiFailure.Http)?.status
                if (!priorUncertain && status in setOf(400, 422)) {
                    journal.clear(saved.owner, origin)
                    pending = null
                    update { copy(pendingRetry = false, pendingReview = null, review = null, notice = RecurrenceNotice.RequestRejected) }
                } else update { copy(notice = RecurrenceNotice.WriteUncertain) }
            }
        }
    }

    private suspend fun nonKeyed(context: BoundContext, request: suspend () -> RepositoryResult<RecurrenceRule>) {
        when (val result = request()) {
            is RepositoryResult.Success -> { if (!active()) return; update { copy(review = null, writeSucceeded = true, notice = null) }; loadResources(context, true) }
            is RepositoryResult.Failure -> { update { copy(review = null, requiresRefresh = true, notice = RecurrenceNotice.RefreshRequired) } }
        }
    }

    /** Decode only the immutable command shapes this session itself may persist. */
    private fun readPendingReview(saved: PendingRecurrence): RecurrenceReview? = runCatching {
        require(saved.owner == owner && saved.origin == origin)
        val command = Json.parseToJsonElement(saved.command).jsonObject
        require(command.keys == setOf("path", "method", "body"))
        require(command.string("method") == "POST")
        val path = command.string("path")
        val body = Json.parseToJsonElement(command.string("body")).jsonObject
        when {
            path == "/v1/recurring-rules" -> RecurrenceReview.CreateRule(readRuleDraft(body))
            path.matches(Regex("/v1/recurring-occurrences/[0-9a-fA-F-]{36}/confirm")) -> {
                val idText = path.removePrefix("/v1/recurring-occurrences/").removeSuffix("/confirm")
                val bits = uuidBits(idText)
                RecurrenceReview.Confirm(OccurrenceId(bits.first, bits.second), readActualDraft(body))
            }
            path.matches(Regex("/v1/recurring-occurrences/[0-9a-fA-F-]{36}/skip")) -> {
                require(body.isEmpty())
                val idText = path.removePrefix("/v1/recurring-occurrences/").removeSuffix("/skip")
                val bits = uuidBits(idText)
                RecurrenceReview.Skip(OccurrenceId(bits.first, bits.second))
            }
            path.matches(Regex("/v1/recurring-occurrences/[0-9a-fA-F-]{36}/postpone")) -> {
                require(body.keys == setOf("due_on"))
                val dueOn = body.string("due_on")
                businessDate(dueOn)
                val idText = path.removePrefix("/v1/recurring-occurrences/").removeSuffix("/postpone")
                val bits = uuidBits(idText)
                RecurrenceReview.Postpone(OccurrenceId(bits.first, bits.second), dueOn)
            }
            else -> error("Unsupported pending recurrence command")
        }
    }.getOrNull()

    private fun readRuleDraft(body: JsonObject): RecurrenceRuleDraft {
        require(body.keys == setOf("amount", "currency", "cadence", "starts_on", "ends_on", "time_zone", "account_id", "category_id", "description"))
        val currency = Currency.valueOf(body.string("currency"))
        val amount = body.string("amount")
        readMoney(JsonPrimitive(amount), currency)
        val cadence = body.string("cadence")
        require(cadence in setOf("weekly", "monthly", "quarterly", "annual"))
        val startsOn = body.string("starts_on").also(::businessDate)
        val endsOn = body.nullableString("ends_on")?.also(::businessDate)
        val zone = body.string("time_zone")
        require(zone.isNotBlank() && zone.length <= 255)
        val account = body.nullableString("account_id")?.let { accountId(it) }
        val category = body.nullableString("category_id")?.let { categoryId(it) }
        return RecurrenceRuleDraft(amount, currency, cadence, startsOn, endsOn, zone, account, category, body.nullableString("description"))
    }

    private fun readActualDraft(body: JsonObject): ActualPaymentDraft {
        require(body.keys == setOf("amount", "currency", "occurred_on", "account_id", "category_id", "description"))
        val currency = Currency.valueOf(body.string("currency"))
        val amount = body.string("amount")
        readMoney(JsonPrimitive(amount), currency)
        val date = body.string("occurred_on").also(::businessDate)
        return ActualPaymentDraft(amount, currency, date, body.nullableString("account_id")?.let { accountId(it) },
            body.nullableString("category_id")?.let { categoryId(it) }, body.nullableString("description"))
    }

    private fun validateRule(draft: RecurrenceRuleDraft, current: RecurrenceRule?) {
        val form = RecurringExpenseForm(draft.amount, draft.currency, draft.cadence, draft.startsOn, draft.endsOn,
            draft.timeZone, draft.description, draft.accountId, draft.categoryId)
        val accounts = state.value.accounts.map { it.account }
        val validation = validateRecurringExpenseForm(form, accounts)
        val issues = validation.issues.filterNot {
            it.field == "accountId" && it.code == "account_not_active" && current?.accountId == draft.accountId
        }
        val category = draft.categoryId?.let { id -> state.value.categories.firstOrNull { it.id == id } }
        val categoryInvalid = draft.categoryId != null && (category == null || category.kind != TransactionKind.EXPENSE)
        if (issues.isNotEmpty() || categoryInvalid) invalid(issues)
        else {
            // Do not force re-selection of an archived account retained from the current rule.
            val selected = draft.accountId?.let { id -> accounts.firstOrNull { it.id == id } }
            val unknownSelection = draft.accountId != null && selected == null && current?.accountId != draft.accountId
            if (unknownSelection) {
                invalid(listOf(ValidationIssue("accountId", "account_not_available")))
                return
            }
            val archivedUnchanged = current?.accountId == draft.accountId && selected?.status == AccountStatus.ARCHIVED
            if (selected?.status == AccountStatus.ARCHIVED && !archivedUnchanged) invalid(listOf(ValidationIssue("accountId", "account_not_active")))
            else {
                val normalized = draft.copy(
                    amount = writeMoney(readMoney(JsonPrimitive(draft.amount), draft.currency)).jsonPrimitive.content,
                    startsOn = wireDate(businessDate(draft.startsOn)),
                    endsOn = draft.endsOn?.let { wireDate(businessDate(it)) },
                )
                update { copy(review = if (current == null) RecurrenceReview.CreateRule(normalized) else RecurrenceReview.EditRule(current.id, normalized), issues = emptyList(), notice = null) }
            }
        }
    }

    private suspend fun validateActual(id: OccurrenceId, actual: ActualPaymentDraft) {
        if (planned(id) == null) return invalid()
        withOwner { currentOwner, context ->
            if (owner != currentOwner || !context.profile.emailVerified) { denied(); return@withOwner }
            val zone = context.profile.timeZone
            if (zone == null) { update { copy(notice = RecurrenceNotice.ZoneRequired) }; return@withOwner }
            val form = TransactionForm(TransactionKind.EXPENSE, actual.amount, actual.currency, actual.occurredOn,
                actual.accountId, actual.categoryId, actual.description)
            val validation = validateTransactionForm(form, state.value.accounts.map { it.account })
            val category = actual.categoryId?.let { id -> state.value.categories.firstOrNull { it.id == id } }
            val categoryInvalid = actual.categoryId != null && (category == null || category.kind != TransactionKind.EXPENSE)
            val todayDate = today(zone)?.let { runCatching { businessDate(it) }.getOrNull() }
            val actualDate = runCatching { businessDate(actual.occurredOn) }.getOrNull()
            val future = todayDate != null && actualDate != null && actualDate > todayDate
            if (!validation.isLocallyValid || categoryInvalid || future || todayDate == null) {
                invalid(validation.issues + if (future) listOf(ValidationIssue("occurredOn", "future_date")) else emptyList())
            } else {
                val normalized = actual.copy(
                    amount = writeMoney(readMoney(JsonPrimitive(actual.amount), actual.currency)).jsonPrimitive.content,
                    occurredOn = wireDate(businessDate(actual.occurredOn)),
                )
                update { copy(review = RecurrenceReview.Confirm(id, normalized), issues = emptyList(), notice = null) }
            }
        }
    }

    private fun planned(id: OccurrenceId) = state.value.occurrences.firstOrNull { it.id == id && it.state == OccurrenceState.PLANNED }
    private fun review(block: () -> Unit) { if (!retired && !state.value.busy && pending == null && !state.value.requiresRefresh) block() }
    private fun invalid(issues: List<ValidationIssue> = emptyList()) { update { copy(review = null, issues = issues, notice = RecurrenceNotice.InvalidForm) } }
    private fun fail(error: ApiFailure, preserve: Boolean) {
        val notice = if ((error as? ApiFailure.Http)?.status in setOf(401, 403)) RecurrenceNotice.AccessDenied
            else when (error) { ApiFailure.Network, ApiFailure.Timeout -> RecurrenceNotice.NetworkUnavailable; else -> RecurrenceNotice.ServiceUnavailable }
        update { copy(notice = notice) }
    }
    private fun denied(): RepositoryResult<Nothing> {
        update { copy(notice = RecurrenceNotice.AccessDenied) }
        return RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
    }
    private fun active() = !retired
    private fun retireForOwnerSwitch() {
        retired = true
        generation++
        owner = null
        pending = null
        mutableState.value = RecurrenceScreenState(notice = RecurrenceNotice.AccessDenied)
    }
    private fun update(block: RecurrenceScreenState.() -> RecurrenceScreenState) { if (!retired) mutableState.value = state.value.block() }

    private data class BoundContext(val profile: OnboardingProfile, val token: String)
    private suspend fun withOwner(block: suspend (OwnerId, BoundContext) -> Unit) {
        val result = session.authorized { token ->
            when (val result = HttpOnboardingRepository(api, token).profile()) {
                is RepositoryResult.Failure -> { fail(result.error, true); result }
                is RepositoryResult.Success -> {
                    val profile = result.value
                    val existing = owner
                    if (!profile.emailVerified || existing != null && existing != profile.ownerId) {
                        denied()
                        if (!profile.emailVerified || existing != null && existing != profile.ownerId) retireForOwnerSwitch()
                        RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                    } else if (!active()) {
                        RepositoryResult.Success(Unit)
                    } else {
                        block(profile.ownerId, BoundContext(profile, token))
                        RepositoryResult.Success(Unit)
                    }
                }
            }
        }
        if (result is RepositoryResult.Failure) fail(result.error, true)
    }

    private suspend fun operation(block: suspend () -> Unit) {
        if (retired || !mutex.tryLock()) return
        val capturedGeneration = generation
        mutableState.value = state.value.copy(busy = true, notice = null, writeSucceeded = false)
        try { block() }
        catch (cancelled: CancellationException) { if (!retired && generation == capturedGeneration) update {
            copy(busy = false, review = if (pending == null) null else review,
                requiresRefresh = requiresRefresh || pending == null &&
                    (review is RecurrenceReview.EditRule || review is RecurrenceReview.PauseRule || review is RecurrenceReview.ResumeRule),
                notice = if (pending != null) RecurrenceNotice.WriteUncertain else RecurrenceNotice.NetworkUnavailable)
        }; throw cancelled }
        catch (_: RecurrenceJournalException) { update { copy(notice = RecurrenceNotice.StorageUnavailable) } }
        catch (_: Exception) { update { copy(notice = RecurrenceNotice.ServiceUnavailable) } }
        finally { if (!retired && generation == capturedGeneration) update { copy(busy = false) }; mutex.unlock() }
    }
}

fun authenticatedRecurrenceSession(
    session: AuthSession,
    api: ApiClient,
    journal: RecurrenceJournal,
    origin: String,
    newKey: () -> IdempotencyKey,
    today: (zone: String) -> String?,
) = AuthenticatedRecurrenceSession(session, api, journal, origin, newKey, today)

private fun actualDraft(body: JsonObject) = ActualPaymentDraft(
    body.string("amount"), Currency.valueOf(body.string("currency")), body.string("occurred_on"),
    body.nullableString("account_id")?.let(::accountId), body.nullableString("category_id")?.let(::categoryId),
    body.nullableString("description"),
)
private fun occurrenceId(text: String) = uuidBits(text).let { OccurrenceId(it.first, it.second) }
private fun ruleBody(draft: RecurrenceRuleDraft): String = buildJsonObject {
    put("amount", draft.amount); put("currency", draft.currency.name); put("cadence", draft.cadence)
    put("starts_on", draft.startsOn); put("ends_on", draft.endsOn?.let(::JsonPrimitive) ?: JsonNull); put("time_zone", draft.timeZone)
    put("account_id", draft.accountId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
    put("category_id", draft.categoryId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
    put("description", draft.description?.let(::JsonPrimitive) ?: JsonNull)
}.toString()
private fun actualBody(draft: ActualPaymentDraft): String = buildJsonObject {
    put("amount", draft.amount); put("currency", draft.currency.name); put("occurred_on", draft.occurredOn)
    put("account_id", draft.accountId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
    put("category_id", draft.categoryId?.let { JsonPrimitive(wireId(it)) } ?: JsonNull)
    put("description", draft.description?.let(::JsonPrimitive) ?: JsonNull)
}.toString()
private fun uuidText(high: Long, low: Long): String {
    val digits = high.toULong().toString(16).padStart(16, '0') + low.toULong().toString(16).padStart(16, '0')
    return "${digits.take(8)}-${digits.substring(8, 12)}-${digits.substring(12, 16)}-${digits.substring(16, 20)}-${digits.drop(20)}"
}
private fun wireId(id: RuleId) = uuidText(id.highBits, id.lowBits)
private fun wireId(id: OccurrenceId) = uuidText(id.highBits, id.lowBits)
