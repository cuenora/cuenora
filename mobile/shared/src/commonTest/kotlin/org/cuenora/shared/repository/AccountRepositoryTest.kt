package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.client.engine.mock.respond
import io.ktor.client.request.HttpRequestData
import io.ktor.client.request.HttpResponseData
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlin.test.*

class AccountRepositoryTest {
    private val requestId = "0123456789abcdef0123456789abcdef"
    private val owner = OwnerId(3, 4)
    private val context = RepositoryContext(owner, "synthetic-bearer")
    private val accountJson = """{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active"}"""

    private fun account() = Account(AccountId(0, 1), owner, AccountName("Daily"), AccountType.NORMAL,
        Currency.SEK, AccountStatus.ACTIVE)

    private fun client(handler: suspend MockRequestHandleScope.(HttpRequestData) -> HttpResponseData): Pair<ApiClient, MockEngine> {
        val engine = MockEngine(handler)
        return ApiClient(engine, "https://example.invalid") to engine
    }

    @Test
    fun listPreservesPaginationArchivedRowsOwnerIdentityAndExactSignedBalances() = runTest {
        var calls = 0
        val body = """{"accounts":[
            {"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active","balance":"92233720368547758.07"},
            {"id":"00000000-0000-0000-0000-000000000002","name":"Old","account_type":"savings","currency":"EUR","status":"archived","balance":"-92233720368547758.07"}
        ],"next_cursor":"00000000-0000-0000-0000-000000000002"}"""
        val (api, engine) = client { request ->
            calls++
            assertEquals("https://example.invalid/v1/accounts?limit=2&cursor=abc%2Fdef", request.url.toString())
            assertEquals("Bearer synthetic-bearer", request.headers[HttpHeaders.Authorization])
            respond(body, HttpStatusCode.OK, headersOf("X-Request-ID", requestId))
        }
        try {
            val result = assertIs<RepositoryResult.Success<Page<AccountWithBalance>>>(
                HttpAccountRepository(api, context).list(AccountListQuery(2, "abc/def")))
            assertEquals("00000000-0000-0000-0000-000000000002", result.value.nextCursor)
            assertEquals(listOf(AccountStatus.ACTIVE, AccountStatus.ARCHIVED), result.value.items.map { it.account.status })
            assertEquals(owner, result.value.items.first().account.ownerId)
            assertEquals(Money(Long.MAX_VALUE, Currency.SEK), result.value.items[0].balance)
            assertEquals(Money(-Long.MAX_VALUE, Currency.EUR), result.value.items[1].balance)
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
        assertFailsWith<IllegalArgumentException> { AccountListQuery(101) }
    }

    @Test
    fun createSendsExactOpeningAndCallerCredentialsAndKey() = runTest {
        var calls = 0
        val (api, engine) = client { request ->
            calls++
            assertEquals(HttpMethod.Post, request.method)
            assertEquals("Bearer synthetic-bearer", request.headers[HttpHeaders.Authorization])
            assertEquals("create_123", request.headers["Idempotency-Key"])
            assertEquals("application/json", request.body.contentType.toString())
            assertEquals(
                """{"name":"Daily","account_type":"normal","currency":"SEK","opening_balance":{"amount":"-92233720368547758.07","occurred_on":"2026-10-01"}}""",
                (request.body as TextContent).text,
            )
            respond(accountJson, HttpStatusCode.Created)
        }
        try {
            val command = CreateAccountCommand(AccountName(" Daily "), AccountType.NORMAL, Currency.SEK,
                OpeningBalance(Money(-Long.MAX_VALUE, Currency.SEK), BusinessDate(2026, 10, 1)))
            assertEquals(RepositoryResult.Success(account()),
                HttpAccountRepository(api, context).create(command, IdempotencyKey("create_123")))
            assertEquals(1, calls)
            assertFailsWith<IllegalArgumentException> {
                CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK,
                    OpeningBalance(Money(1, Currency.EUR), BusinessDate(2026, 10, 1)))
            }
        } finally { api.close(); engine.close() }
    }

    @Test
    fun updateEncodesNameAndArchiveStatusWithoutMutationKey() = runTest {
        val archived = accountJson.replace("\"name\":\"Daily\"", "\"name\":\"Renamed\"")
            .replace("\"status\":\"active\"", "\"status\":\"archived\"")
        var calls = 0
        val (api, engine) = client { request ->
            calls++
            assertEquals(HttpMethod.Patch, request.method)
            assertEquals("https://example.invalid/v1/accounts/00000000-0000-0000-0000-000000000001", request.url.toString())
            assertEquals("Bearer synthetic-bearer", request.headers[HttpHeaders.Authorization])
            assertNull(request.headers["Idempotency-Key"])
            assertEquals("""{"name":"Renamed","status":"archived"}""", (request.body as TextContent).text)
            respond(archived, HttpStatusCode.OK)
        }
        try {
            val command = UpdateAccountCommand(AccountId(0, 1), PatchField.Value(AccountName("Renamed")),
                PatchField.Value(AccountStatus.ARCHIVED))
            val result = assertIs<RepositoryResult.Success<Account>>(HttpAccountRepository(api, context).update(command))
            assertEquals(AccountStatus.ARCHIVED, result.value.status)
            assertEquals(1, calls)
            assertFailsWith<IllegalArgumentException> { UpdateAccountCommand(AccountId(0, 1)) }
            assertFailsWith<IllegalArgumentException> {
                UpdateAccountCommand(AccountId(0, 1), name = PatchField.Null)
            }
        } finally { api.close(); engine.close() }
    }

    @Test
    fun unknownResourceFieldsAreIgnoredAcrossAccountResponses() = runTest {
        val item = """{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active","balance":"1.00","future_account_field":true}"""
        val page = """{"accounts":[$item],"next_cursor":null,"future_page_field":{"v":1}}"""
        val (listApi, listEngine) = client { respond(page, HttpStatusCode.OK) }
        try {
            assertIs<RepositoryResult.Success<Page<AccountWithBalance>>>(HttpAccountRepository(listApi, context).list())
        } finally { listApi.close(); listEngine.close() }

        val (createApi, createEngine) = client {
            respond(accountJson.replace("\"status\":\"active\"", "\"status\":\"active\",\"future_field\":true"), HttpStatusCode.Created)
        }
        try {
            assertIs<RepositoryResult.Success<Account>>(HttpAccountRepository(createApi, context).create(
                CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("key")))
        } finally { createApi.close(); createEngine.close() }
    }

    @Test
    fun mutationResponsesMustMatchRequestedAccountAndAssignments() = runTest {
        val otherId = accountJson.replace("00000000-0000-0000-0000-000000000001", "00000000-0000-0000-0000-000000000002")
        val (updateApi, updateEngine) = client { respond(otherId, HttpStatusCode.OK) }
        try {
            assertEquals(RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null)),
                HttpAccountRepository(updateApi, context).update(UpdateAccountCommand(AccountId(0, 1),
                    status = PatchField.Value(AccountStatus.ARCHIVED))))
        } finally { updateApi.close(); updateEngine.close() }

        val archived = accountJson.replace("\"status\":\"active\"", "\"status\":\"archived\"")
        val (wrongActionApi, wrongActionEngine) = client { respond(accountJson, HttpStatusCode.OK) }
        try {
            assertEquals(RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null)),
                HttpAccountRepository(wrongActionApi, context).update(UpdateAccountCommand(AccountId(0, 1),
                    status = PatchField.Value(AccountStatus.ARCHIVED))))
        } finally { wrongActionApi.close(); wrongActionEngine.close() }

        val mismatches = listOf(
            accountJson.replace("\"status\":\"active\"", "\"status\":\"archived\""),
            accountJson.replace("\"currency\":\"SEK\"", "\"currency\":\"EUR\""),
            accountJson.replace("\"account_type\":\"normal\"", "\"account_type\":\"savings\""),
            accountJson.replace("\"name\":\"Daily\"", "\"name\":\"Other\""),
        )
        for (body in mismatches) {
            val (api, engine) = client { respond(body, HttpStatusCode.Created) }
            try {
                assertEquals(RepositoryResult.Failure(ApiFailure.InvalidResponse(201, null)),
                    HttpAccountRepository(api, context).create(
                        CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("key")))
            } finally { api.close(); engine.close() }
        }
    }

    @Test
    fun httpFailureIsPreservedAndNoAutomaticRetryOccurs() = runTest {
        var calls = 0
        val body = """{"code":"future_account_error","message":"Safe message.","details":null,"request_id":"$requestId"}"""
        val (api, engine) = client {
            calls++
            respond(body, HttpStatusCode.Conflict, headersOf("X-Request-ID", requestId))
        }
        try {
            assertEquals(RepositoryResult.Failure(ApiFailure.Http(409,
                ApiError("future_account_error", "Safe message.", requestId), requestId, null)),
                HttpAccountRepository(api, context).update(UpdateAccountCommand(AccountId(0, 1),
                    status = PatchField.Value(AccountStatus.ARCHIVED))))
            assertEquals(1, calls)
        } finally { api.close(); engine.close() }
    }

    @Test
    fun cancellationPropagatesThroughRepository() = runTest {
        val engine = MockEngine { throw CancellationException("cancelled") }
        val api = ApiClient(engine)
        try {
            assertFailsWith<CancellationException> { HttpAccountRepository(api, context).list() }
        } finally { api.close(); engine.close() }
    }

    @Test
    fun malformedSuccessUnknownEnumsAndNumericAmountsBecomeRedactedFailures() = runTest {
        val malformed = listOf(
            "not-json",
            """{"accounts":[]}""",
            """{"accounts":[{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active","balance":12.34}],"next_cursor":null}""",
            """{"accounts":[{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"unknown","currency":"SEK","status":"active","balance":"1.00"}],"next_cursor":null}""",
        )
        for (body in malformed) {
            val (api, engine) = client { respond(body, HttpStatusCode.OK, headersOf("X-Request-ID", requestId)) }
            try {
                val failure = assertIs<RepositoryResult.Failure>(HttpAccountRepository(api, context).list()).error
                assertEquals(ApiFailure.InvalidResponse(200, requestId), failure)
                assertFalse(failure.toString().contains("do-not-retain"))
            } finally { api.close(); engine.close() }
        }
        val (api, engine) = client { respond(accountJson, HttpStatusCode.OK) }
        try {
            assertEquals(RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null)),
                HttpAccountRepository(api, context).create(CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK),
                    IdempotencyKey("key")))
        } finally { api.close(); engine.close() }
    }

    @Test
    fun fakeScriptsSuccessFailureAndCapturesPublicCalls() = runTest {
        val page = Page(listOf(AccountWithBalance(account(), Money(-25, Currency.SEK))), "next")
        val failure = RepositoryResult.Failure(ApiFailure.Timeout)
        val fake = FakeAccountRepository(
            listResults = mutableListOf(RepositoryResult.Success(page)),
            createResults = mutableListOf(failure),
            updateResults = mutableListOf(RepositoryResult.Success(account())),
        )
        val query = AccountListQuery(5, "cursor")
        val command = CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK)
        val key = IdempotencyKey("key")
        assertEquals(RepositoryResult.Success(page), fake.list(query))
        assertEquals(failure, fake.create(command, key))
        val update = UpdateAccountCommand(AccountId(0, 1), status = PatchField.Value(AccountStatus.ACTIVE))
        assertEquals(RepositoryResult.Success(account()), fake.update(update))
        assertEquals(listOf(FakeAccountRepository.Call.List(query), FakeAccountRepository.Call.Create(command, key),
            FakeAccountRepository.Call.Update(update)), fake.calls)
    }
}
