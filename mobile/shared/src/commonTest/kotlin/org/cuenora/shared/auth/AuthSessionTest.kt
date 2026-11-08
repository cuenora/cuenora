package org.cuenora.shared.auth

import kotlinx.coroutines.*
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.repository.RepositoryResult
import org.cuenora.shared.session.*
import kotlin.test.*

class AuthSessionTest {
    private fun grant(suffix: String = "1") = TokenGrant(SessionCredentials("access-$suffix", "refresh-$suffix"), 900)

    private class Store : SessionStore {
        var value: SessionCredentials? = null
        var failRead = false
        var failClear = false
        var failReplace = false
        override suspend fun read(): SessionCredentials? {
            if (failRead) throw SessionStoreException(SessionFailure.Corrupt)
            return value
        }
        override suspend fun clear() {
            if (failClear) throw SessionStoreException(SessionFailure.Unavailable)
            value = null
        }
        override suspend fun replace(credentials: SessionCredentials) {
            if (failReplace) throw SessionStoreException(SessionFailure.Unavailable)
            value = credentials
        }
    }
    private class Repository : AuthRepository {
        var logins = 0
        var registrations = 0
        var refreshes = 0
        var revoked: String? = null
        var next: RepositoryResult<TokenGrant> = RepositoryResult.Success(TokenGrant(SessionCredentials("access-2", "refresh-2"), 900))
        var onRefresh: suspend () -> Unit = {}
        var onLogin: suspend () -> Unit = {}
        var onLogout: suspend () -> Unit = {}
        override suspend fun register(email: String, password: String): RepositoryResult<Unit> {
            registrations++; return RepositoryResult.Success(Unit)
        }
        override suspend fun login(email: String, password: String): RepositoryResult<TokenGrant> {
            logins++; onLogin(); return next
        }
        override suspend fun refresh(credentials: SessionCredentials): RepositoryResult<TokenGrant> {
            refreshes++; onRefresh(); return next
        }
        override suspend fun logout(credentials: SessionCredentials): RepositoryResult<Unit> {
            revoked = credentials.refreshToken; onLogout(); return RepositoryResult.Success(Unit)
        }
    }

    @Test fun loginAndRegistrationUseDifferentValidationAndNeverCreateAuthorityFromAcceptance() = runTest {
        val store = Store(); val repo = Repository(); val session = AuthSession(repo, store) { 0L }
        session.restore()
        session.register("not-email", "short")
        assertEquals(0, repo.registrations)
        assertTrue(session.state.value.issues.isNotEmpty())
        session.register("person@example.test", "a synthetic password")
        assertEquals(AuthNotice.RegistrationAccepted, session.state.value.notice)
        assertNull(store.value)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
        session.login("person@example.test", "x")
        assertEquals(1, repo.logins)
        assertEquals("refresh-2", store.value?.refreshToken)
        assertEquals(AuthPhase.SignedIn, session.state.value.phase)
    }

    @Test fun restartRotatesOnceAndDoesNotTrustPersistedAccessOrInventOwnerIdentity() = runTest {
        val store = Store().apply { value = grant().credentials }; val repo = Repository()
        repo.onRefresh = { assertNull(store.value) }
        val session = AuthSession(repo, store) { 0L }
        session.restore(); session.restore()
        assertEquals(1, repo.refreshes)
        assertEquals("refresh-2", store.value?.refreshToken)
        assertEquals(AuthPhase.SignedIn, session.state.value.phase)
        assertFalse(session.state.value.toString().contains("access-2"))
    }

    @Test fun simultaneousRenewalsUseOnlyOneRotationAndLogoutRevokesTheNewPair() = runTest {
        var now = 0L
        val store = Store(); val repo = Repository(); val session = AuthSession(repo, store) { now }
        session.login("person@example.test", "x")
        now = 900_000
        repo.next = RepositoryResult.Success(grant("3"))
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        repo.onRefresh = { entered.complete(Unit); release.await() }
        val first = async { session.renewIfNeeded() }; entered.await()
        val second = async { session.renewIfNeeded() }
        release.complete(Unit); first.await(); second.await()
        assertEquals(1, repo.refreshes)
        session.logout()
        assertEquals("refresh-3", repo.revoked)
        assertNull(store.value)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
    }

    @Test fun renewalFailureAndCancellationCannotReplayAnOldPairAfterRestart() = runTest {
        for (cancel in listOf(false, true)) {
            val store = Store().apply { value = grant().credentials }; val repo = Repository()
            if (cancel) repo.onRefresh = { throw CancellationException("synthetic") }
            else repo.next = RepositoryResult.Failure(ApiFailure.Timeout)
            val session = AuthSession(repo, store) { 0L }
            if (cancel) assertFailsWith<CancellationException> { session.restore() } else session.restore()
            assertNull(store.value)
            assertEquals(AuthPhase.SignedOut, session.state.value.phase)
            AuthSession(repo, store) { 0L }.restore()
            assertEquals(1, repo.refreshes)
        }
    }

    @Test fun corruptOrUnavailableStorageNeverBecomesSuccessfulLoginOrLogout() = runTest {
        val store = Store().apply { failRead = true }; val repo = Repository()
        val session = AuthSession(repo, store) { 0L }
        session.restore()
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        assertEquals(0, repo.refreshes)
        store.failRead = false; session.logout()
        store.failReplace = true; session.login("person@example.test", "x")
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        store.failReplace = false; session.logout(); session.login("person@example.test", "x")
        store.failClear = true; session.logout()
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        assertNotNull(store.value)
        store.failClear = false; session.logout()
        assertNull(store.value)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
    }

    @Test fun offlineLogoutClearsLocalCredentialsButDoesNotClaimRemoteRevocation() = runTest {
        val store = Store(); val repo = Repository(); val session = AuthSession(repo, store) { 0L }
        session.login("person@example.test", "x")
        repo.onLogout = { throw CancellationException("synthetic") }
        assertFailsWith<CancellationException> { session.logout() }
        assertNull(store.value)
        assertEquals(AuthNotice.RemoteLogoutUnconfirmed, session.state.value.notice)
    }

    @Test fun logoutWaitsForInFlightRefreshAndThenLeavesNoRestorableCredentials() = runTest {
        var now = 0L
        val store = Store(); val repo = Repository(); val session = AuthSession(repo, store) { now }
        session.login("person@example.test", "x"); now = 900_000
        repo.next = RepositoryResult.Success(grant("3"))
        repo.next = RepositoryResult.Success(grant("3"))
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        repo.onRefresh = { entered.complete(Unit); release.await() }
        val refresh = async { session.renewIfNeeded() }; entered.await()
        val logout = async { session.logout() }; release.complete(Unit)
        refresh.await(); logout.await()
        assertNull(store.value)
        assertEquals("refresh-3", repo.revoked)
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
    }
    @Test fun refreshIsNotSentWhenRemovingTheConsumedGenerationWouldFail() = runTest {
        val store = Store().apply { value = grant().credentials; failClear = true }
        val repo = Repository(); val session = AuthSession(repo, store) { 0L }
        session.restore()
        assertEquals(0, repo.refreshes)
        assertEquals(AuthPhase.StorageBlocked, session.state.value.phase)
        assertEquals("refresh-1", store.value?.refreshToken)
    }

    @Test fun rejectedRefreshRequiresLoginAndNoRetry() = runTest {
        val store = Store().apply { value = grant().credentials }; val repo = Repository()
        repo.next = RepositoryResult.Failure(ApiFailure.Http(401, null, null, null))
        val session = AuthSession(repo, store) { 0L }
        session.restore(); session.renewIfNeeded(); session.restore()
        assertEquals(AuthNotice.SessionExpired, session.state.value.notice)
        assertEquals(1, repo.refreshes)
        assertNull(store.value)
    }

    @Test fun cancellationDuringLoginLeavesNeitherCredentialsNorBusyState() = runTest {
        val store = Store(); val repo = Repository()
        repo.onLogin = { throw CancellationException("synthetic") }
        val session = AuthSession(repo, store) { 0L }
        assertFailsWith<CancellationException> { session.login("person@example.test", "x") }
        assertEquals(AuthPhase.SignedOut, session.state.value.phase)
        assertNull(store.value)
    }

    @Test fun renewalUsesTheExpirySafetyMarginWithoutRefreshingEarly() = runTest {
        var now = 0L
        val repo = Repository(); val store = Store(); val session = AuthSession(repo, store) { now }
        session.login("person@example.test", "x")
        now = 869_999; session.renewIfNeeded()
        assertEquals(0, repo.refreshes)
        now = 870_000; session.renewIfNeeded()
        assertEquals(1, repo.refreshes)
        assertEquals(AuthPhase.SignedIn, session.state.value.phase)
    }

}
