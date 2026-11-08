package org.cuenora.shared.onboarding

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.auth.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthenticatedOnboardingBackendTest {
    @Test fun changedOwnerOrUnverifiedProfileCannotPostOrChangeZoneUsingStaleUi() = runTest {
        for ((id, verified) in listOf(8 to true, 7 to false)) {
            var writes = 0
            val engine = MockEngine { request ->
                if (request.method != HttpMethod.Get) writes++
                respond("""{"id":"00000000-0000-0000-0000-00000000000$id","email_verified":$verified,"time_zone":null}""", HttpStatusCode.OK)
            }
            val api = ApiClient(engine, "https://example.invalid")
            val session = loginSession()
            try {
                val backend = AuthenticatedOnboardingBackend(session, api)
                val pending = PendingAccount(OwnerId(0, 7), CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK), IdempotencyKey("stable-key"))
                assertIs<RepositoryResult.Failure>(backend.create(pending))
                assertIs<RepositoryResult.Failure>(backend.confirmZone(OwnerId(0, 7), "Europe/Stockholm"))
                assertEquals(0, writes)
            } finally { api.close(); engine.close() }
        }
    }
    private suspend fun loginSession(): AuthSession {
        val repository = object : AuthRepository {
            override suspend fun register(email: String, password: String) = RepositoryResult.Success(Unit)
            override suspend fun login(email: String, password: String) = RepositoryResult.Success(TokenGrant(SessionCredentials("access", "refresh"), 900))
            override suspend fun refresh(credentials: SessionCredentials) = login("", "")
            override suspend fun logout(credentials: SessionCredentials) = RepositoryResult.Success(Unit)
        }
        val store = object : SessionStore {
            override suspend fun read(): SessionCredentials? = null
            override suspend fun replace(credentials: SessionCredentials) {}
            override suspend fun clear() {}
        }
        return AuthSession(repository, store) { 0L }.also { it.login("person@example.test", "x") }
    }
}
