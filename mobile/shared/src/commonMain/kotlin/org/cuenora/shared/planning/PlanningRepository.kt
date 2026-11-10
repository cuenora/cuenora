package org.cuenora.shared.planning

import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.repository.*
import org.cuenora.shared.serialization.ReportAmountSerializer

interface PlanningReader {
    suspend fun load(through: BusinessDate? = null): RepositoryResult<AccountPlanning>
}

class HttpPlanningRepository(private val api: ApiClient, private val context: RepositoryContext) : PlanningReader {
    override suspend fun load(through: BusinessDate?): RepositoryResult<AccountPlanning> = mapResponse(
        api.request(queryPath("/v1/reports/account-planning", listOf("through" to through?.let(::wireDate))), HttpMethod.Get, headers = context.headers()), 200,
        ) { parsePlanning(it.jsonObject, through) }
}

internal fun parsePlanning(root: JsonObject, expectedThrough: BusinessDate? = null): AccountPlanning {
    root.only("as_of", "through", "complete", "accounts", "unassigned")
    val asOf = root.string("as_of")
    require(Regex("[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\\.[0-9]+)?(?:Z|[+-][0-9]{2}:[0-9]{2})").matches(asOf))
    businessDate(asOf.take(10))
    require(asOf.substring(11, 13).toInt() <= 23 && asOf.substring(14, 16).toInt() <= 59 && asOf.substring(17, 19).toInt() <= 60)
    val through = businessDate(root.string("through"))
    require(expectedThrough == null || expectedThrough == through)
    val complete = root.getValue("complete").strictBoolean()
    val accountRows = root.getValue("accounts").jsonArray
    require(accountRows.size <= 1000)
    val accounts = accountRows.map { value ->
        val row = value.jsonObject
        row.only("account_id", "name", "account_type", "status", "currency", "current_balance", "pending_total", "projected_total", "planning_difference", "complete")
        val currency = Currency.valueOf(row.string("currency"))
        val balance = amount(row.getValue("current_balance"), currency)
        val pending = amount(row.getValue("pending_total"), currency)
        val projected = amount(row.getValue("projected_total"), currency)
        val rowComplete = row.getValue("complete").strictBoolean()
        val difference = row.nullableString("planning_difference")?.also(ExactDecimal::requireValid)
        require(rowComplete || difference == null)
        if (rowComplete) require(difference == ExactDecimal.subtract(ExactDecimal.subtract(balance, pending), projected))
        PlanningAccount(accountId(row.string("account_id")), AccountName(row.string("name")).value, when (row.string("account_type")) {
            "normal" -> AccountType.NORMAL; "savings" -> AccountType.SAVINGS; else -> error("Invalid account type")
        }, when (row.string("status")) { "active" -> AccountStatus.ACTIVE; "archived" -> AccountStatus.ARCHIVED; else -> error("Invalid account status") },
            currency, balance, pending, projected, difference, rowComplete)
    }
    require(accounts.map { it.id }.toSet().size == accounts.size)
    val currencies = listOf(Currency.SEK, Currency.EUR, Currency.USD)
    val unassigned = root.getValue("unassigned").jsonArray.map { value ->
        val row = value.jsonObject
        row.only("currency", "pending_total", "projected_total", "complete")
        val currency = Currency.valueOf(row.string("currency"))
        UnassignedPlanning(currency, amount(row.getValue("pending_total"), currency), amount(row.getValue("projected_total"), currency), row.getValue("complete").strictBoolean())
    }
    require(unassigned.map { it.currency } == currencies)
    require(complete == (accounts.all { it.complete } && unassigned.all { it.complete }))
    return AccountPlanning(asOf, through, complete, accounts, unassigned)
}

private fun amount(value: JsonElement, currency: Currency): String =
    Json.decodeFromJsonElement(ReportAmountSerializer, value).also(ExactDecimal::requireValid)

private fun JsonElement.strictBoolean(): Boolean {
    val value = jsonPrimitive
    require(!value.isString && value.content in setOf("true", "false"))
    return value.boolean
}

private fun JsonObject.only(vararg keys: String) {
    require(this.keys == keys.toSet()) { "Unexpected or missing report field" }
}
