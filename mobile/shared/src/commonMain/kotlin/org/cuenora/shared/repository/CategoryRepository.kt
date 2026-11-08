package org.cuenora.shared.repository

import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.CategoryId
import org.cuenora.shared.domain.TransactionKind
import org.cuenora.shared.network.ApiClient

data class CatalogCategory(
    val id: CategoryId,
    val code: String,
    val kind: TransactionKind,
    val name: String,
)

interface CategoryRepository {
    suspend fun list(): RepositoryResult<List<CatalogCategory>>
}

/** HTTP adapter for the authenticated global read-only category catalog. */
class HttpCategoryRepository(
    private val api: ApiClient,
    private val context: RepositoryContext,
) : CategoryRepository {
    override suspend fun list(): RepositoryResult<List<CatalogCategory>> =
        mapResponse(api.request("/v1/categories", HttpMethod.Get, headers = context.headers()), 200) { element ->
            val root = element.jsonObject
            require(root.keys == setOf("categories")) { "Invalid category catalog" }
            val categories = root.getValue("categories").jsonArray.map { item ->
                val value = item.jsonObject
                require(value.keys == setOf("id", "code", "kind", "name")) { "Invalid category representation" }
                val code = value.string("code")
                require(categoryCode.matches(code)) { "Invalid category code" }
                val name = value.string("name")
                require(validCategoryName(name)) { "Invalid category name" }
                CatalogCategory(
                    id = categoryId(value.string("id")),
                    code = code,
                    kind = when (value.string("kind")) {
                        "income" -> TransactionKind.INCOME
                        "expense" -> TransactionKind.EXPENSE
                        else -> throw IllegalArgumentException("Invalid category kind")
                    },
                    name = name,
                )
            }
            require(categories.size == 10) { "Category catalog must be complete" }
            require(categories.map { it.id }.toSet().size == categories.size) { "Duplicate category identity" }
            require(categories.map { it.code }.toSet().size == categories.size) { "Duplicate category code" }
            require(categories.map { wireId(it.id) } == categories.map { wireId(it.id) }.sorted()) {
                "Category catalog is not ordered by UUID"
            }
            categories
        }

    private companion object {
        val categoryCode = Regex("[a-z]+(?:_[a-z]+)*")
    }
}

private fun validCategoryName(name: String): Boolean {
    if (name.none { !it.isWhitespace() } || name.any { it.isISOControl() }) return false
    var codePoints = 0
    var index = 0
    while (index < name.length) {
        val first = name[index]
        if (first.isHighSurrogate() && index + 1 < name.length && name[index + 1].isLowSurrogate()) index++
        codePoints++
        if (codePoints > 255) return false
        index++
    }
    return codePoints > 0
}
