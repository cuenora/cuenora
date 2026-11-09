package org.cuenora.shared.recurrence

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import org.cuenora.shared.validation.ValidationIssue
import kotlin.test.*

class RecurrenceSessionTest {
    private class Store : SessionStore {
        var credentials: SessionCredentials? = null
        override suspend fun read() = credentials
        override suspend fun replace(credentials: SessionCredentials) { this.credentials = credentials }
        override suspend fun clear() { credentials = null }
    }
    private class Journal : RecurrenceJournal {
        var saved: PendingRecurrence? = null
        override suspend fun read(owner: OwnerId, origin: String) = saved?.takeIf { it.owner == owner && it.origin == origin }
        override suspend fun save(owner: OwnerId, origin: String, pending: PendingRecurrence) { check(saved == null || saved?.encode() == pending.encode()); saved = pending }
        override suspend fun clear(owner: OwnerId, origin: String) { saved = null }
    }
    private fun auth(): AuthSession {
        val store = Store()
        return AuthSession(object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("synthetic-access", "synthetic-refresh"), 900))
            override suspend fun refresh(credentials: SessionCredentials) = login("", "")
            override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
        }, store) { 0L }
    }
    private val categories = (1..10).joinToString(",") { number ->
        val id = number.toString().padStart(12, '0')
        "{\"id\":\"00000000-0000-0000-0000-$id\",\"code\":\"category_${('a'.code + number - 1).toChar()}\",\"kind\":\"expense\",\"name\":\"Category $number\"}"
    }

    @Test fun invalidRuleReviewNeverSendsMutation() = runTest {
        val methods = mutableListOf<Pair<String, String>>()
        val engine = MockEngine { request ->
            methods += request.method.value to request.url.encodedPath
            val body = when (request.url.encodedPath) {
                "/v1/me" -> """{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}"""
                "/v1/accounts" -> """{"accounts":[],"next_cursor":null}"""
                "/v1/categories" -> """{"categories":[$categories]}"""
                "/v1/recurring-rules" -> """{"rules":[],"next_cursor":null}"""
                "/v1/recurring-occurrences" -> """{"occurrences":[],"next_cursor":null}"""
                else -> error("Unexpected request")
            }
            respond(body, HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val session = auth(); session.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(session, api, Journal(), "https://example.invalid", { IdempotencyKey("key_1") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("not-money", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm"))
            assertEquals(RecurrenceNotice.InvalidForm, recurrence.state.value.notice)
            assertNull(recurrence.state.value.review)
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm", accountId = AccountId(0, 99)))
            assertEquals(RecurrenceNotice.InvalidForm, recurrence.state.value.notice)
            assertNull(recurrence.state.value.review, "Unknown account IDs must fail closed")
            recurrence.submitReviewed()
            assertTrue(methods.none { it.second == "/v1/recurring-rules" && it.first == "POST" })
        } finally { api.close(); engine.close() }
    }

    @Test fun uncertainCreateRetriesOnlyTheOriginalStoredKeyAndBody() = runTest {
        val posts = mutableListOf<Pair<String, String>>()
        var failFirst = true
        val rule = """{"id":"00000000-0000-0000-0000-000000000010","amount":"12.34","currency":"SEK","cadence":"monthly","starts_on":"2026-11-01","ends_on":null,"time_zone":"Europe/Stockholm","account_id":null,"category_id":null,"description":null,"state":"active","paused_on":null,"generated_through":null}"""
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> if (request.method == HttpMethod.Post) {
                    val body = (request.body as io.ktor.http.content.TextContent).text
                    posts += request.headers["Idempotency-Key"]!! to body
                    if (failFirst) {
                        failFirst = false
                        respond("""{"code":"recurrence_unavailable","message":"Unavailable","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.ServiceUnavailable)
                    } else respond(rule, HttpStatusCode.Created)
                } else respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        val journal = Journal()
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("create_original") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm"))
            assertIs<RecurrenceReview.CreateRule>(recurrence.state.value.review)
            recurrence.submitReviewed()
            assertEquals(1, posts.size)
            assertTrue(recurrence.state.value.pendingRetry)
            assertEquals(posts.first().first, journal.saved?.key?.value)
            assertNotNull(journal.saved)
            recurrence.clear()
            val restored = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("must_not_replace_original") }) { "2026-11-09" }
            restored.load()
            assertTrue(restored.state.value.pendingRetry)
            assertEquals(RecurrenceReview.CreateRule(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm")), restored.state.value.pendingReview)
            restored.retryPending()
            assertEquals(2, posts.size)
            assertEquals(posts.first(), posts.last())
            assertNull(journal.saved)
            assertFalse(restored.state.value.pendingRetry)
            assertNull(restored.state.value.pendingReview)
            assertTrue(restored.state.value.writeSucceeded)
        } finally { api.close(); engine.close() }
    }


    @Test fun clearRetiresAnInFlightProfileReadWithoutStartingResourceReads() = runTest {
        val started = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        var resourceReads = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/me") {
                started.complete(Unit)
                release.await()
                respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
            } else {
                resourceReads++
                error("Retired session must not start resource reads")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, Journal(), "https://example.invalid", { IdempotencyKey("unused") }) { "2026-11-09" }
            val loading = async { recurrence.load() }
            started.await()
            recurrence.clear()
            release.complete(Unit)
            loading.await()
            assertEquals(0, resourceReads)
            assertEquals(RecurrenceScreenState(), recurrence.state.value)
        } finally { api.close(); engine.close() }
    }


    @Test fun uncertainNonKeyedRuleEditRequiresAnExplicitFreshRead() = runTest {
        var patches = 0
        val rule = """{"id":"00000000-0000-0000-0000-000000000010","amount":"12.34","currency":"SEK","cadence":"monthly","starts_on":"2026-11-01","ends_on":null,"time_zone":"Europe/Stockholm","account_id":null,"category_id":null,"description":null,"state":"active","paused_on":null,"generated_through":null}"""
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> respond("""{"rules":[$rule],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-rules/00000000-0000-0000-0000-000000000010" -> {
                    patches++
                    assertEquals(HttpMethod.Patch, request.method)
                    assertNull(request.headers["Idempotency-Key"])
                    respond("""{"code":"recurrence_unavailable","message":"Unavailable","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.ServiceUnavailable)
                }
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, Journal(), "https://example.invalid", { IdempotencyKey("unused") }) { "2026-11-09" }
            recurrence.load()
            assertEquals(1, recurrence.state.value.rules.size, "notice=${recurrence.state.value.notice} accounts=${recurrence.state.value.accounts.size}")
            recurrence.prepareEdit(RuleId(0, 16), RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm", description = "Updated"))
            assertEquals(emptyList(), recurrence.state.value.issues)
            assertIs<RecurrenceReview.EditRule>(recurrence.state.value.review)
            recurrence.submitReviewed()
            assertEquals(1, patches)
            assertTrue(recurrence.state.value.requiresRefresh)
            recurrence.submitReviewed()
            assertEquals(1, patches)
            recurrence.refresh()
            assertFalse(recurrence.state.value.requiresRefresh)
            assertEquals(1, patches)
        } finally { api.close(); engine.close() }
    }


    @Test fun initialDefinitiveValidationRejectionClearsNewPendingCommand() = runTest {
        val journal = Journal()
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> if (request.method == HttpMethod.Post)
                    respond("""{"code":"invalid_request","message":"Invalid","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.BadRequest)
                else respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("rejected_initial") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null, "Europe/Stockholm"))
            recurrence.submitReviewed()
            assertNull(journal.saved)
            assertFalse(recurrence.state.value.pendingRetry)
            assertNull(recurrence.state.value.pendingReview)
            assertEquals(RecurrenceNotice.RequestRejected, recurrence.state.value.notice)
        } finally { api.close(); engine.close() }
    }

    @Test fun validationRejectionOnRetryRetainsPreviouslyUncertainCommand() = runTest {
        val journal = Journal()
        var posts = 0
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> if (request.method == HttpMethod.Post) {
                    posts++
                    if (posts == 1) respond("""{"code":"recurrence_unavailable","message":"Unavailable","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.ServiceUnavailable)
                    else respond("""{"code":"invalid_request","message":"Invalid","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.UnprocessableEntity)
                } else respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("retry_uncertain") }) { "2026-11-09" }
            recurrence.load()
            val reviewed = RecurrenceReview.CreateRule(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null, "Europe/Stockholm"))
            recurrence.prepareCreate(reviewed.terms)
            recurrence.submitReviewed()
            assertNotNull(journal.saved)
            recurrence.clear()
            val restored = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { error("Must not generate replacement key") }) { "2026-11-09" }
            restored.load()
            assertEquals(reviewed, restored.state.value.pendingReview)
            restored.retryPending()
            assertNotNull(journal.saved)
            assertTrue(restored.state.value.pendingRetry)
            assertEquals(reviewed, restored.state.value.pendingReview)
            assertEquals(RecurrenceNotice.WriteUncertain, restored.state.value.notice)
        } finally { api.close(); engine.close() }
    }

    @Test fun loadsEveryAccountPageAndRefreshResetsPreviouslyUsedRuleCursors() = runTest {
        val firstAccount = """{"id":"00000000-0000-0000-0000-000000000001","name":"First","account_type":"normal","currency":"SEK","status":"active","balance":"0.00"}"""
        val secondAccount = """{"id":"00000000-0000-0000-0000-000000000002","name":"Second","account_type":"normal","currency":"SEK","status":"active","balance":"0.00"}"""
        val ruleRequests = mutableListOf<String?>()
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> if (request.url.parameters["cursor"] == null)
                    respond("""{"accounts":[$firstAccount],"next_cursor":"accounts/next"}""", HttpStatusCode.OK)
                else {
                    assertEquals("accounts/next", request.url.parameters["cursor"])
                    respond("""{"accounts":[$secondAccount],"next_cursor":null}""", HttpStatusCode.OK)
                }
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> {
                    ruleRequests += request.url.parameters["cursor"]
                    if (request.url.parameters["cursor"] == "rules/next") respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                    else respond("""{"rules":[],"next_cursor":"rules/next"}""", HttpStatusCode.OK)
                }
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, Journal(), "https://example.invalid", { IdempotencyKey("unused") }) { "2026-11-09" }
            recurrence.load()
            assertEquals(2, recurrence.state.value.accounts.size)
            recurrence.loadMoreRules()
            assertEquals(listOf(null, "rules/next"), ruleRequests)
            recurrence.refresh()
            assertEquals(listOf(null, "rules/next", null), ruleRequests)
            assertTrue(recurrence.state.value.hasMoreRules)
            assertEquals(null, recurrence.state.value.notice)
        } finally { api.close(); engine.close() }
    }

    @Test fun ruleEditPauseAndResumeReviewChangedAccountWithoutIdempotencyKeys() = runTest {
        val accountOne = "00000000-0000-0000-0000-000000000001"
        val accountTwo = "00000000-0000-0000-0000-000000000002"
        var assigned = accountOne
        var ruleState = "active"
        var paused = "null"
        val actions = mutableListOf<String>()
        fun currentRule() = """{"id":"00000000-0000-0000-0000-000000000010","amount":"12.34","currency":"SEK","cadence":"monthly","starts_on":"2026-11-01","ends_on":null,"time_zone":"Europe/Stockholm","account_id":"$assigned","category_id":null,"description":null,"state":"$ruleState","paused_on":$paused,"generated_through":null}"""
        val accounts = listOf(accountOne to "First", accountTwo to "Second").joinToString(",") { (id, name) ->
            """{"id":"$id","name":"$name","account_type":"normal","currency":"SEK","status":"active","balance":"0.00"}"""
        }
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[$accounts],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> respond("""{"rules":[${currentRule()}],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-rules/00000000-0000-0000-0000-000000000010" -> {
                    assertEquals(HttpMethod.Patch, request.method)
                    assertNull(request.headers["Idempotency-Key"])
                    assertEquals("{\"account_id\":\"$accountTwo\"}", (request.body as io.ktor.http.content.TextContent).text)
                    assigned = accountTwo; actions += "edit"
                    respond(currentRule(), HttpStatusCode.OK)
                }
                "/v1/recurring-rules/00000000-0000-0000-0000-000000000010/pause" -> {
                    assertEquals(HttpMethod.Post, request.method); assertNull(request.headers["Idempotency-Key"])
                    ruleState = "paused"; paused = "\"2026-11-09\""; actions += "pause"
                    respond(currentRule(), HttpStatusCode.OK)
                }
                "/v1/recurring-rules/00000000-0000-0000-0000-000000000010/resume" -> {
                    assertEquals(HttpMethod.Post, request.method); assertNull(request.headers["Idempotency-Key"])
                    ruleState = "active"; paused = "null"; actions += "resume"
                    respond(currentRule(), HttpStatusCode.OK)
                }
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, Journal(), "https://example.invalid", { IdempotencyKey("unused") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareEdit(RuleId(0, 16), RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm", accountId = AccountId(0, 2)))
            assertIs<RecurrenceReview.EditRule>(recurrence.state.value.review)
            recurrence.submitReviewed()
            assertEquals(AccountId(0, 2), recurrence.state.value.rules.single().accountId)
            recurrence.preparePause(RuleId(0, 16)); assertIs<RecurrenceReview.PauseRule>(recurrence.state.value.review)
            recurrence.submitReviewed(); assertEquals(RuleState.PAUSED, recurrence.state.value.rules.single().state)
            recurrence.prepareResume(RuleId(0, 16)); assertIs<RecurrenceReview.ResumeRule>(recurrence.state.value.review)
            recurrence.submitReviewed(); assertEquals(RuleState.ACTIVE, recurrence.state.value.rules.single().state)
            assertEquals(listOf("edit", "pause", "resume"), actions)
        } finally { api.close(); engine.close() }
    }

    @Test fun postponeMustBeStrictlyLaterAndRetriesOnlyOriginalCommand() = runTest {
        val posts = mutableListOf<Pair<String?, String>>()
        var failFirst = true
        val occurrence = """{"id":"00000000-0000-0000-0000-000000000021","rule_id":"00000000-0000-0000-0000-000000000010","scheduled_on":"2026-01-31","due_on":"2026-02-05","state":"planned","amount":"12.34","currency":"SEK","account_id":null,"category_id":null,"description":null,"transaction_id":null}"""
        val action = """{"id":"00000000-0000-0000-0000-000000000021","scheduled_on":"2026-01-31","due_on":"2026-02-06","state":"planned"}"""
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[$occurrence],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences/00000000-0000-0000-0000-000000000021/postpone" -> {
                    posts += request.headers["Idempotency-Key"] to (request.body as io.ktor.http.content.TextContent).text
                    if (failFirst) {
                        failFirst = false
                        respond("""{"code":"recurrence_unavailable","message":"Unavailable","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.ServiceUnavailable)
                    } else respond(action, HttpStatusCode.OK)
                }
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        val journal = Journal()
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("postpone_original") }) { "2026-11-09" }
            recurrence.load()
            recurrence.preparePostpone(OccurrenceId(0, 33), "2026-02-05")
            assertEquals(RecurrenceNotice.InvalidForm, recurrence.state.value.notice)
            assertNull(recurrence.state.value.review)
            recurrence.preparePostpone(OccurrenceId(0, 33), "2026-02-04")
            assertEquals(RecurrenceNotice.InvalidForm, recurrence.state.value.notice)
            recurrence.preparePostpone(OccurrenceId(0, 33), "2026-02-06")
            assertEquals(RecurrenceReview.Postpone(OccurrenceId(0, 33), "2026-02-06"), recurrence.state.value.review)
            recurrence.submitReviewed()
            assertTrue(recurrence.state.value.pendingRetry)
            assertIs<RecurrenceReview.Postpone>(recurrence.state.value.pendingReview)
            recurrence.retryPending()
            assertEquals(2, posts.size)
            assertEquals(posts.first(), posts.last())
            assertNull(journal.saved)
        } finally { api.close(); engine.close() }
    }

    @Test fun skipIsTerminalAndRetryReplaysTheSameReviewedCommand() = runTest {
        val posts = mutableListOf<Pair<String?, String>>()
        var failFirst = true
        var occurrenceState = "planned"
        val occurrence = """{"id":"00000000-0000-0000-0000-000000000021","rule_id":"00000000-0000-0000-0000-000000000010","scheduled_on":"2026-01-31","due_on":"2026-02-05","state":"planned","amount":"12.34","currency":"SEK","account_id":null,"category_id":null,"description":null,"transaction_id":null}"""
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[${occurrence.replace("\"state\":\"planned\"", "\"state\":\"$occurrenceState\"")}],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences/00000000-0000-0000-0000-000000000021/skip" -> {
                    posts += request.headers["Idempotency-Key"] to (request.body as io.ktor.http.content.TextContent).text
                    if (failFirst) {
                        failFirst = false
                        respond("""{"code":"recurrence_unavailable","message":"Unavailable","details":null,"request_id":"0123456789abcdef0123456789abcdef"}""", HttpStatusCode.ServiceUnavailable)
                    } else {
                        occurrenceState = "skipped"
                        respond("""{"id":"00000000-0000-0000-0000-000000000021","scheduled_on":"2026-01-31","due_on":"2026-02-05","state":"skipped"}""", HttpStatusCode.OK)
                    }
                }
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        val journal = Journal()
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("skip_original") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareSkip(OccurrenceId(0, 33))
            assertEquals(RecurrenceReview.Skip(OccurrenceId(0, 33)), recurrence.state.value.review)
            recurrence.submitReviewed()
            assertTrue(recurrence.state.value.pendingRetry)
            assertIs<RecurrenceReview.Skip>(recurrence.state.value.pendingReview)
            recurrence.retryPending()
            assertEquals(OccurrenceState.SKIPPED, recurrence.state.value.occurrences.single().state)
            assertEquals(2, posts.size)
            assertEquals(posts.first(), posts.last())
            assertNull(journal.saved)
        } finally { api.close(); engine.close() }
    }

    @Test fun newRuleRejectsUnknownAccountInsteadOfTreatingItAsAccountless() = runTest {
        val engine = MockEngine { request ->
            when (request.url.encodedPath) {
                "/v1/me" -> respond("""{"id":"00000000-0000-0000-0000-000000000007","email_verified":true,"time_zone":"Europe/Stockholm"}""", HttpStatusCode.OK)
                "/v1/accounts" -> respond("""{"accounts":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/categories" -> respond("""{"categories":[$categories]}""", HttpStatusCode.OK)
                "/v1/recurring-rules" -> respond("""{"rules":[],"next_cursor":null}""", HttpStatusCode.OK)
                "/v1/recurring-occurrences" -> respond("""{"occurrences":[],"next_cursor":null}""", HttpStatusCode.OK)
                else -> error("Unexpected request")
            }
        }
        val api = ApiClient(engine, "https://example.invalid")
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, Journal(), "https://example.invalid", { IdempotencyKey("unused") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm", accountId = AccountId(0, 999)))
            assertNull(recurrence.state.value.review)
            assertEquals(listOf(ValidationIssue("accountId", "account_not_available")), recurrence.state.value.issues)
        } finally { api.close(); engine.close() }
    }

}
