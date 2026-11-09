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
import kotlin.test.*

class RecurrenceRetirementTest {
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

    @Test fun retiredDuringJournalSaveMustNotStartMutation() = runTest {
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
        val saving = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val journal = object : RecurrenceJournal {
            var saved: PendingRecurrence? = null
            override suspend fun read(owner: OwnerId, origin: String) = saved
            override suspend fun save(owner: OwnerId, origin: String, pending: PendingRecurrence) {
                saved = pending; saving.complete(Unit); release.await()
            }
            override suspend fun clear(owner: OwnerId, origin: String) { saved = null }
        }
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("create_original") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm"))
            assertIs<RecurrenceReview.CreateRule>(recurrence.state.value.review)
            val submitting = async { recurrence.submitReviewed() }
            saving.await()
            recurrence.clear()
            release.complete(Unit)
            submitting.await()
            assertEquals(RecurrenceScreenState(), recurrence.state.value)
            assertNotNull(journal.saved)
            assertEquals(0, posts.size, "Retired presentation must not start a write after suspended storage returns")
            val preserved = journal.saved!!.encode()
            val preservedBody = kotlinx.serialization.json.Json.parseToJsonElement(journal.saved!!.command).let {
                (it as kotlinx.serialization.json.JsonObject)["body"]!!.let { value -> (value as kotlinx.serialization.json.JsonPrimitive).content }
            }
            val restored = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid",
                { IdempotencyKey("must_not_replace_original") }) { "2026-11-09" }
            restored.load()
            assertTrue(restored.state.value.pendingRetry)
            assertEquals(RecurrenceReview.CreateRule(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm")), restored.state.value.pendingReview)
            assertEquals(preserved, journal.saved!!.encode())
            assertEquals(0, posts.size)
            restored.retryPending()
            assertEquals(1, posts.size)
            assertEquals("create_original", posts.single().first)
            assertEquals(preservedBody, posts.single().second)
            assertEquals(preserved, journal.saved!!.encode())
            assertNotNull(journal.saved)

        } finally { api.close(); engine.close() }
    }


    @Test fun parallelSubmitDoesNotStartSecondCommand() = runTest {
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
        val saving = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val journal = object : RecurrenceJournal {
            var saved: PendingRecurrence? = null
            override suspend fun read(owner: OwnerId, origin: String) = saved
            override suspend fun save(owner: OwnerId, origin: String, pending: PendingRecurrence) {
                saved = pending; saving.complete(Unit); release.await()
            }
            override suspend fun clear(owner: OwnerId, origin: String) { saved = null }
        }
        try {
            val auth = auth(); auth.login("person@example.test", "secret")
            val recurrence = authenticatedRecurrenceSession(auth, api, journal, "https://example.invalid", { IdempotencyKey("create_original") }) { "2026-11-09" }
            recurrence.load()
            recurrence.prepareCreate(RecurrenceRuleDraft("12.34", Currency.SEK, "monthly", "2026-11-01", null,
                "Europe/Stockholm"))
            assertIs<RecurrenceReview.CreateRule>(recurrence.state.value.review)
            val submitting = async { recurrence.submitReviewed() }
            saving.await()
            recurrence.submitReviewed()
            assertEquals(0, posts.size)
            release.complete(Unit)
            submitting.await()
            assertTrue(recurrence.state.value.pendingRetry)
            assertNotNull(journal.saved)
            assertEquals(1, posts.size, "Concurrent submit must not start a second operation")
        } finally { api.close(); engine.close() }
    }


}
