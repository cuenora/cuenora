package org.cuenora.shared.repository

import io.ktor.client.engine.mock.*
import io.ktor.http.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.*
import kotlin.test.*

class CategoryRepositoryTest {
    private val owner = OwnerId(1, 2)
    private val token = "fixture-token"
    private val context get() = RepositoryContext(owner, token)
    private val codes = listOf("salary", "interest", "other_income", "housing", "groceries", "transport",
        "utilities", "health", "leisure", "other_expense")

    private val catalog = (1..10).joinToString(",") { suffix ->
        val kind = if (suffix <= 3) "income" else "expense"
        """{"id":"10000000-0000-4000-8000-${suffix.toString().padStart(12, '0')}","code":"${codes[suffix - 1]}","kind":"$kind","name":"Category $suffix"}"""
    }

    @Test fun listsStrictCatalogUsingAuthenticatedReadOnlyGet() = runTest {
        var method: HttpMethod? = null
        var path = ""
        var authorization: String? = null
        val engine = MockEngine { request ->
            method = request.method
            path = request.url.encodedPathAndQuery
            authorization = request.headers[HttpHeaders.Authorization]
            respond("""{"categories":[$catalog]}""", HttpStatusCode.OK,
                headersOf(HttpHeaders.ContentType, "application/json"))
        }
        val api = ApiClient(engine, "http://localhost")
        val categories = assertIs<RepositoryResult.Success<List<CatalogCategory>>>(
            HttpCategoryRepository(api, context).list()
        ).value
        assertEquals(10, categories.size)
        assertEquals(CategoryId(0x1000000000004000L, 0x8000000000000001uL.toLong()), categories.first().id)
        assertEquals("salary", categories.first().code)
        assertEquals(TransactionKind.INCOME, categories.first().kind)
        assertEquals("Category 1", categories.first().name)
        assertEquals(HttpMethod.Get, method)
        assertEquals("/v1/categories", path)
        assertEquals("Bearer $token", authorization)
        api.close()
    }

    @Test fun rejectsInvalidShapeKindsCodesAndDuplicateIdentitiesOrCodes() = runTest {
        val valid = """{"categories":[$catalog]}"""
        val variants = listOf(
            valid.replace("\"salary\"", "\"Bad-Code\""),
            valid.replace("\"kind\":\"income\"", "\"kind\":\"transfer\""),
            valid.replace("10000000-0000-4000-8000-000000000002", "10000000-0000-4000-8000-000000000001"),
            valid.replace("\"interest\"", "\"salary\""),
            """{"categories":[${catalog.substringBeforeLast(",")}]}""",
            valid.replace("\"name\":\"Category 1\"", "\"name\":\"Category 1\",\"extra\":true"),
            valid.replace("\"name\":\"Category 1\"", "\"name\":\"\""),
            valid.replace("\"name\":\"Category 1\"", "\"name\":\"   \""),
            valid.replace("\"name\":\"Category 1\"", "\"name\":\"${"x".repeat(256)}\""),
            valid.replace("\"name\":\"Category 1\"", "\"name\":\"\\u0001\""),
        )
        for (payload in variants) {
            val api = ApiClient(MockEngine { _ -> respond(payload, HttpStatusCode.OK) }, "http://localhost")
            assertIs<RepositoryResult.Failure>(HttpCategoryRepository(api, context).list())
            api.close()
        }
    }

    @Test fun mapsHttpFailureAndRethrowsCancellation() = runTest {
        val api = ApiClient(MockEngine { _ -> respond("{}", HttpStatusCode.Unauthorized) }, "http://localhost")
        val failure = assertIs<RepositoryResult.Failure>(HttpCategoryRepository(api, context).list()).error
        assertEquals(ApiFailure.Http(401, null, null, null), failure)
        api.close()

        val cancelled = ApiClient(MockEngine { throw CancellationException("cancelled") }, "http://localhost")
        assertFailsWith<CancellationException> { HttpCategoryRepository(cancelled, context).list() }
        cancelled.close()
    }
}
