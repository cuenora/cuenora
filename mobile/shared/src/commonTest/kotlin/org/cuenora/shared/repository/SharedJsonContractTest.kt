package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import kotlin.test.*

class SharedJsonContractTest {
    private val context = RepositoryContext(OwnerId(3, 4), "synthetic-bearer")
    private val fixtures get() = Json.parseToJsonElement(listingFixtures).jsonObject

    private fun client(name: String, path: String): ApiClient = ApiClient(MockEngine { request ->
        assertEquals(HttpMethod.Get, request.method)
        assertEquals(path, request.url.encodedPath)
        assertEquals("Bearer synthetic-bearer", request.headers[HttpHeaders.Authorization])
        respond(fixtures.getValue(name).toString(), HttpStatusCode.OK,
            headersOf(HttpHeaders.ContentType, "application/json"))
    }, "https://example.invalid")

    @Test fun sharedAccountListingPreservesExactBalancesMetadataAndCursor() = runTest {
        val api = client("accounts", "/v1/accounts")
        try {
            val page = assertIs<RepositoryResult.Success<Page<AccountWithBalance>>>(
                HttpAccountRepository(api, context).list()).value
            assertEquals(listOf(
                AccountWithBalance(Account(AccountId(0, 1), context.ownerId, AccountName("Daily"),
                    AccountType.NORMAL, Currency.SEK, AccountStatus.ACTIVE), Money(Long.MAX_VALUE, Currency.SEK)),
                AccountWithBalance(Account(AccountId(0, 2), context.ownerId, AccountName("Old savings"),
                    AccountType.SAVINGS, Currency.EUR, AccountStatus.ARCHIVED), Money(-Long.MAX_VALUE, Currency.EUR)),
            ), page.items)
            assertEquals("00000000-0000-0000-0000-000000000002", page.nextCursor)
        } finally { api.close() }
    }

    @Test fun sharedAccountlessIncomePreservesNullsAndAmountBeyondDoublePrecision() = runTest {
        val api = client("accountless", "/v1/transactions")
        try {
            val page = assertIs<RepositoryResult.Success<Page<TransactionRecord>>>(
                HttpTransactionRepository(api, context).list()).value
            assertEquals(1, page.items.size)
            val record = page.items.single()
            assertEquals(Transaction(TransactionId(0, 3), TransactionKind.INCOME, TransactionState.POSTED,
                Money(9_007_199_254_740_993, Currency.USD), BusinessDate(2024, 2, 29)), record.transaction)
            assertNull(record.quotedRate)
            assertNull(record.rateComparison)
            assertNull(page.nextCursor)
        } finally { api.close() }
    }

    @Test fun sharedTransferPreservesActualAmountsAndFlattenedQuoteComparison() = runTest {
        val api = client("transfer", "/v1/transactions")
        try {
            val page = assertIs<RepositoryResult.Success<Page<TransactionRecord>>>(
                HttpTransactionRepository(api, context).list()).value
            assertEquals(1, page.items.size)
            val record = page.items.single()
            assertEquals(Transaction(TransactionId(0, 4), TransactionKind.TRANSFER, TransactionState.POSTED,
                Money(10_000, Currency.SEK), BusinessDate(2024, 2, 29), AccountId(0, 1),
                description = "Currency exchange", destinationAccountId = AccountId(0, 2),
                receivedAmount = Money(1_234, Currency.EUR)), record.transaction)
            assertEquals("0.1", record.quotedRate)
            assertEquals(RateComparison(Money(1_000, Currency.EUR), Money(234, Currency.EUR), true), record.rateComparison)
            assertNull(page.nextCursor)
        } finally { api.close() }
    }
}
