package org.cuenora.shared.accounts

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import io.ktor.http.content.TextContent
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class AuthenticatedAccountArchiveTest {
    private class Store : SessionStore {
        var value: SessionCredentials? = null
        override suspend fun read() = value
        override suspend fun replace(credentials: SessionCredentials) { value = credentials }
        override suspend fun clear() { value = null }
    }

    private fun session() = AuthSession(object : AuthRepository {
        override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
        override suspend fun login(email: String, password: String) = RepositoryResult.Success(
            TokenGrant(SessionCredentials("archive-access", "archive-refresh"), 900))
        override suspend fun refresh(credentials: SessionCredentials) = login("", "")
        override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
    }, Store()) { 0L }

    private fun account(owner: Int = 7, status: AccountStatus = AccountStatus.ACTIVE) = Account(
        AccountId(0, 4), OwnerId(0, owner.toLong()), AccountName("Daily"), AccountType.NORMAL, Currency.SEK, status,
    )

    private fun profile(owner: Int = 7, verified: Boolean = true) =
        """{"id":"00000000-0000-0000-0000-${owner.toString().padStart(12, '0')}","email_verified":$verified,"time_zone":null}"""

    private fun response(status: String) =
        """{"id":"00000000-0000-0000-0000-000000000004","name":"Daily","account_type":"normal","currency":"SEK","status":"$status"}"""

    @Test fun archiveUsesVerifiedCurrentOwnerAndExistingAbsoluteStatusEndpoint() = runTest {
        val requests = mutableListOf<Pair<String, String?>>()
        val engine = MockEngine { request ->
            requests += request.method.value to request.url.encodedPath
            assertEquals("Bearer archive-access", request.headers[HttpHeaders.Authorization])
            if (request.url.encodedPath == "/v1/me") respond(profile(), HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
            else {
                assertEquals(HttpMethod.Patch, request.method)
                assertEquals("""{"status":"archived"}""", (request.body as TextContent).text)
                respond(response("archived"), HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val manager = AuthenticatedAccountArchive(auth, api)
            val result = manager.update(account(), AccountStatus.ARCHIVED)
            assertEquals(AccountStatus.ARCHIVED, assertIs<RepositoryResult.Success<Account>>(result).value.status)
            assertEquals(listOf<Pair<String, String?>>("GET" to "/v1/me",
                "PATCH" to "/v1/accounts/00000000-0000-0000-0000-000000000004"), requests)
        } finally { api.close(); engine.close() }
    }

    @Test fun unverifiedOrDifferentOwnerNeverIssuesAccountMutation() = runTest {
        var currentProfile = profile(verified = false)
        var patches = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/me") respond(currentProfile, HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
            else { patches++; respond(response("archived"), HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json")) }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val manager = AuthenticatedAccountArchive(auth, api)
            assertIs<RepositoryResult.Failure>(manager.update(account(), AccountStatus.ARCHIVED))
            currentProfile = profile(owner = 8)
            assertIs<RepositoryResult.Failure>(manager.update(account(), AccountStatus.ARCHIVED))
            assertEquals(0, patches)
        } finally { api.close(); engine.close() }
    }

    @Test fun clearDuringProfileReadPreventsLateMutation() = runTest {
        val profileEntered = CompletableDeferred<Unit>()
        val releaseProfile = CompletableDeferred<Unit>()
        var patches = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/me") {
                profileEntered.complete(Unit); releaseProfile.await()
                respond(profile(), HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            } else { patches++; respond(response("archived"), HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json")) }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val manager = AuthenticatedAccountArchive(auth, api)
            val result = async { manager.update(account(), AccountStatus.ARCHIVED) }
            profileEntered.await(); manager.clear(); releaseProfile.complete(Unit)
            assertIs<RepositoryResult.Failure>(result.await())
            assertEquals(0, patches)
        } finally { api.close(); engine.close() }
    }

    @Test fun serverRejectionIsReturnedWithoutRetryAndReactivationUsesActiveStatus() = runTest {
        var patches = 0
        val engine = MockEngine { request ->
            if (request.url.encodedPath == "/v1/me") respond(profile(), HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
            else {
                patches++
                if (patches == 1) respond("{}", HttpStatusCode.Conflict)
                else respond(response("active"), HttpStatusCode.OK, headersOf(HttpHeaders.ContentType, "application/json"))
            }
        }
        val api = ApiClient(engine, "http://localhost")
        try {
            val auth = session(); auth.login("person@example.test", "x")
            val manager = AuthenticatedAccountArchive(auth, api)
            val rejected = manager.update(account(), AccountStatus.ARCHIVED)
            assertEquals(409, (assertIs<RepositoryResult.Failure>(rejected).error as ApiFailure.Http).status)
            assertEquals(1, patches)
            val reactivated = manager.update(account(status = AccountStatus.ARCHIVED), AccountStatus.ACTIVE)
            assertEquals(AccountStatus.ACTIVE, assertIs<RepositoryResult.Success<Account>>(reactivated).value.status)
            assertEquals(2, patches)
        } finally { api.close(); engine.close() }
    }
}
