package org.cuenora.shared.recording

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.OnboardingProfile
import org.cuenora.shared.repository.*
import org.cuenora.shared.validation.TransactionForm
import kotlin.test.*

@OptIn(ExperimentalCoroutinesApi::class)
class TransferRecordingTest {
    private val owner = OwnerId(0, 7)
    private val source = Account(AccountId(0, 1), owner, AccountName("Daily"), AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE)
    private val destination = source.copy(id = AccountId(0, 2), name = AccountName("Savings"), type = AccountType.SAVINGS, currency = Currency.EUR)
    private val form = TransactionForm(TransactionKind.TRANSFER, "200.00", Currency.SEK, "2026-11-09",
        accountId = source.id, destinationAccountId = destination.id, receivedAmount = "17.95", receivedCurrency = Currency.EUR,
        quotedRate = "000.0900", description = "Synthetic transfer")
    private class Store : PendingTransactionStore {
        var encoded: String? = null
        var unavailable = false
        override suspend fun read(owner: OwnerId) = encoded?.let(PendingTransaction::decode)
        override suspend fun save(pending: PendingTransaction) {
            check(!unavailable)
            check(encoded == null || encoded == pending.encode())
            encoded = pending.encode()
        }
        override suspend fun clear(owner: OwnerId) { encoded = null }
    }
    private inner class Backend : RecordingBackend {
        var accounts = listOf(source, destination)
        var profile = OnboardingProfile(owner, true, "Europe/Stockholm")
        var accountsUnavailable = false
        var result: RepositoryResult<Unit> = RepositoryResult.Success(Unit)
        val sent = mutableListOf<PendingTransaction>()
        var gate: CompletableDeferred<Unit>? = null
        override suspend fun profile() = RepositoryResult.Success(profile)
        override suspend fun confirmZone(owner: OwnerId, zone: String) = RepositoryResult.Success(zone)
        override suspend fun accounts(owner: OwnerId): RepositoryResult<List<Account>> =
            if (accountsUnavailable) RepositoryResult.Failure(ApiFailure.Network) else RepositoryResult.Success(accounts)
        override suspend fun create(pending: PendingTransaction): RepositoryResult<Unit> {
            sent += pending
            gate?.await()
            return result
        }
    }
    private fun flow(backend: Backend, store: Store) = RecordingSession(backend, store,
        { RepositoryResult.Success(emptyList()) }, { IdempotencyKey("transfer-original-key") }, { "2026-11-09" })

    @Test fun transferRequiresExplicitReviewAndSavesActualAmountsNotTheQuote() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store)
        flow.begin(source); assertEquals(source.id, flow.state.value.form.accountId)
        flow.update(form); flow.save(); assertTrue(backend.sent.isEmpty()); assertNull(store.encoded)
        flow.review(); assertEquals(RecordingStage.Review, flow.state.value.stage)
        flow.save(); assertEquals(RecordingStage.Saved, flow.state.value.stage); assertNull(store.encoded)
        val posted = backend.sent.single().command as TransactionCreateCommand.Transfer
        assertEquals(Money(20000, Currency.SEK), posted.amount)
        assertEquals(Money(1795, Currency.EUR), posted.receivedAmount)
        assertEquals(source.id, posted.accountId); assertEquals(destination.id, posted.destinationAccountId)
        assertEquals("000.0900", posted.quotedRate); assertEquals(form.description, posted.description)
    }

    @Test fun transferJournalRoundTripsAndLegacyIncomeExpenseBytesRemainVersionOne() {
        val command = TransactionCreateCommand.Transfer(Money(20000, Currency.SEK), source.id, Money(1795, Currency.EUR),
            destination.id, BusinessDate(2026, 11, 9), "000.0900", "Synthetic transfer")
        for (quote in listOf(null, "000.0900", "0.000000000001")) {
            val original = PendingTransaction(owner, command.copy(quotedRate = quote), IdempotencyKey("transfer-key"))
            val recovered = PendingTransaction.decode(original.encode())
            assertEquals(original.encode(), recovered.encode()); assertEquals(original.command, recovered.command)
            assertEquals(original.owner, recovered.owner); assertEquals(original.key, recovered.key)
            assertFalse(recovered.toString().contains("Synthetic"))
        }
        // Literal v1 payload is a compatibility fixture, not produced by the candidate encoder.
        for (kind in listOf("income", "expense")) {
            val legacy = """{"version":1,"owner_high":"0","owner_low":"7","key":"legacy-key","kind":"$kind","amount":"12.34","currency":"SEK","date":"2026-11-09","account_id":null,"category_id":null,"description":null}"""
            assertEquals(legacy, PendingTransaction.decode(legacy).encode())
        }
    }

    @Test fun transferJournalRejectsMissingExtraIncompatibleOrCorruptedFields() {
        val command = TransactionCreateCommand.Transfer(Money(20000, Currency.SEK), source.id, Money(1795, Currency.EUR),
            destination.id, BusinessDate(2026, 11, 9), "0.09")
        val encoded = PendingTransaction(owner, command, IdempotencyKey("key")).encode()
        for (invalid in listOf(encoded.replace("\"version\":2", "\"version\":1"),
            encoded.replace("\"version\":2", "\"version\":3"), encoded.replace("\"kind\":\"transfer\"", "\"kind\":\"income\""),
            encoded.replace("\"received_amount\":\"17.95\"", "\"received_amount\":\"0.00\""),
            encoded.replace("\"quoted_rate\":\"0.09\"", "\"quoted_rate\":\"0.1234567890123\""),
            encoded.replace("\"category_id\":null", "\"category_id\":\"00000000-0000-0000-0000-000000000009\""),
            encoded.dropLast(1) + ",\"extra\":true}")) {
            assertFailsWith<IllegalArgumentException> { PendingTransaction.decode(invalid) }
        }
    }

    @Test fun invalidTransferDetailsCannotReachReviewOrWrite() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store); flow.begin()
        val invalids = listOf(
            form.copy(accountId = null), form.copy(destinationAccountId = null), form.copy(destinationAccountId = source.id),
            form.copy(destinationAccountId = AccountId(0, 99)), form.copy(accountId = AccountId(0, 99)),
            form.copy(receivedCurrency = Currency.USD), form.copy(currency = Currency.USD),
            form.copy(categoryId = CategoryId(0, 1)), form.copy(occurredOn = "2026-11-10"), form.copy(occurredOn = "2026-02-30"),
        ) + listOf(null, "", "0", "-1", "18.001", "92233720368547758.08").map { form.copy(receivedAmount = it) } +
            listOf("", "0", "-1", "18.001", "92233720368547758.08").map { form.copy(amount = it) } +
            listOf("0", "-0.09", "+0.09", " 0.09", "0,09", "9e-2", ".09", "0.", "NaN", "0.1234567890123", "0.0900000000000")
                .map { form.copy(quotedRate = it) }
        for (invalid in invalids) {
            flow.update(invalid); flow.review(); flow.save()
            assertEquals(RecordingStage.Form, flow.state.value.stage, invalid.toString())
            assertTrue(flow.state.value.issues.isNotEmpty())
            assertTrue(backend.sent.isEmpty()); assertNull(store.encoded)
        }
        for (items in listOf(listOf(source, destination.copy(status = AccountStatus.ARCHIVED)),
                listOf(source.copy(status = AccountStatus.ARCHIVED), destination))) {
            backend.accounts = items; flow.cancel(); flow.begin(); flow.update(form); flow.review()
            assertEquals(RecordingStage.Form, flow.state.value.stage); assertTrue(backend.sent.isEmpty())
        }
        backend.accountsUnavailable = true; flow.cancel(); flow.begin(); flow.update(form); flow.review()
        assertEquals(RecordingStage.Form, flow.state.value.stage)
    }

    @Test fun sameCurrencyRequiresEqualAmountsAndSavingsCanSendWithoutBalanceRestriction() = runTest {
        val backend = Backend(); backend.accounts = listOf(source.copy(type = AccountType.SAVINGS), destination.copy(currency = Currency.SEK))
        val flow = flow(backend, Store()); flow.begin()
        flow.update(form.copy(receivedCurrency = Currency.SEK, receivedAmount = "199.99", quotedRate = null)); flow.review()
        assertEquals(RecordingStage.Form, flow.state.value.stage)
        flow.update(form.copy(receivedCurrency = Currency.SEK, receivedAmount = "200.00", quotedRate = "0.09")); flow.review()
        assertEquals(RecordingStage.Review, flow.state.value.stage)
        flow.save(); assertEquals(RecordingStage.Saved, flow.state.value.stage)
        assertEquals(1, backend.sent.size)
    }

    @Test fun uncertainTransferRestoresExactCommandWithoutReloadingEligibilityOrAutomaticReplay() = runTest {
        val backend = Backend(); backend.result = RepositoryResult.Failure(ApiFailure.Timeout)
        val store = Store(); val first = flow(backend, store)
        first.begin(); first.update(form); first.review(); first.save()
        assertEquals(RecordingStage.Uncertain, first.state.value.stage)
        val original = store.encoded!!
        first.update(form.copy(receivedAmount = "99")); first.cancel(); assertEquals(original, store.encoded)
        backend.accounts = emptyList()
        val restored = flow(backend, store); restored.begin()
        assertEquals(1, backend.sent.size); assertEquals(RecordingStage.Uncertain, restored.state.value.stage)
        assertEquals("17.95", restored.state.value.form.receivedAmount)
        assertEquals(form.destinationAccountId, restored.state.value.form.destinationAccountId)
        assertEquals(form.quotedRate, restored.state.value.form.quotedRate)
        backend.result = RepositoryResult.Success(Unit); restored.save()
        assertEquals(2, backend.sent.size); assertEquals(backend.sent[0].encode(), backend.sent[1].encode())
        assertNull(store.encoded); assertEquals(RecordingStage.Saved, restored.state.value.stage)
    }

    @Test fun transferStorageFailureDuplicateSubmitAndRetirementPreserveWriteSafety() = runTest {
        val backend = Backend(); val store = Store(); store.unavailable = true
        val blocked = flow(backend, store); blocked.begin(); blocked.update(form); blocked.review(); blocked.save()
        assertTrue(backend.sent.isEmpty()); assertEquals(RecordingStage.Uncertain, blocked.state.value.stage)
        store.unavailable = false; val flow = flow(backend, store)
        flow.begin(); flow.update(form); flow.review(); backend.gate = CompletableDeferred()
        val job = launch { flow.save() }; runCurrent(); flow.save(); assertEquals(1, backend.sent.size)
        flow.clear(); backend.gate!!.complete(Unit); job.join()
        assertEquals(RecordingStage.Closed, flow.state.value.stage); assertNotNull(store.encoded)
    }

    @Test fun publicReviewMatchesIndependentDecimalOracleWithoutPostingOrReplacingActuals() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store); flow.begin()
        data class Vector(val sent: String, val quote: String, val indicative: Long)
        val vectors = listOf(
            Vector("1", "0.125", 12), Vector("1", "0.135", 14), Vector("200", "000.0900", 1800),
            Vector("92233720368547758.07", "0.000000000001", 9223372),
            Vector("92233720368547758.07", "1", Long.MAX_VALUE),
            Vector("1234567.89", "0.012345678901", 1524158),
            Vector("0.01", "0.5", 0), Vector("0.03", "0.5", 2),
            Vector("99.99", "0.000000000001", 0), Vector("999.99", "1.000000000001", 99999),
        )
        for (vector in vectors) {
            val input = form.copy(amount = vector.sent, quotedRate = vector.quote, receivedAmount = "0.01")
            flow.update(input); flow.review()
            assertEquals(RecordingStage.Review, flow.state.value.stage)
            val comparison = assertNotNull(flow.state.value.rateComparison)
            assertEquals(vector.indicative, comparison.indicativeReceivedAmount.minorUnits)
            assertEquals(1L - vector.indicative, comparison.difference.minorUnits)
            assertEquals(input, flow.state.value.form); assertTrue(backend.sent.isEmpty()); assertNull(store.encoded)
            flow.edit()
        }
        for (quote in listOf("100000000000000000000000000000000000000", "92233720368547758.08")) {
            val before = store.encoded
            flow.update(form.copy(quotedRate = quote)); flow.review(); flow.save()
            assertEquals(RecordingStage.Form, flow.state.value.stage)
            assertTrue(flow.state.value.issues.any { it.field == "quotedRate" })
            assertTrue(backend.sent.isEmpty()); assertEquals(before, store.encoded)
        }
    }

    @Test fun legacyIncomeAndExpenseRecoveryPreservePublicCommandKeyAndBytes() = runTest {
        for (kind in listOf(TransactionKind.INCOME, TransactionKind.EXPENSE)) {
            val backend = Backend(); val store = Store()
            backend.result = RepositoryResult.Failure(ApiFailure.Timeout)
            val first = flow(backend, store); first.begin()
            first.update(TransactionForm(kind, "12.34", Currency.SEK, "2026-11-09")); first.review(); first.save()
            assertEquals(RecordingStage.Uncertain, first.state.value.stage)
            val original = store.encoded!!
            val restored = flow(backend, store); restored.begin()
            assertEquals(1, backend.sent.size); restored.save()
            assertEquals(2, backend.sent.size); assertEquals(original, store.encoded)
            assertEquals(backend.sent[0].encode(), backend.sent[1].encode())
            assertEquals(kind, restored.state.value.form.kind); assertEquals("12.34", restored.state.value.form.amount)
            assertEquals(RecordingStage.Uncertain, restored.state.value.stage)
        }
    }

    @Test fun cancellationAndIdentityPrerequisitesNeverPostTransfer() = runTest {
        val backend = Backend(); val store = Store(); val flow = flow(backend, store)
        flow.begin(); flow.update(form); flow.review(); flow.cancel()
        assertEquals(RecordingStage.Closed, flow.state.value.stage); assertTrue(backend.sent.isEmpty()); assertNull(store.encoded)
        for (profile in listOf(OnboardingProfile(owner, false, "Europe/Stockholm"), OnboardingProfile(owner, true, null))) {
            backend.profile = profile; val blocked = flow(backend, Store()); blocked.begin(); blocked.update(form); blocked.review(); blocked.save()
            assertEquals(RecordingStage.Blocked, blocked.state.value.stage); assertTrue(backend.sent.isEmpty())
        }
        backend.profile = OnboardingProfile(owner, true, "Europe/Stockholm")
        backend.accounts = listOf(source, destination.copy(ownerId = OwnerId(0, 99)))
        val foreign = flow(backend, Store()); foreign.begin(); assertEquals(RecordingStage.Failed, foreign.state.value.stage)
        assertTrue(backend.sent.isEmpty())
    }
}
