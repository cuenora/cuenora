package org.cuenora.shared.insights

import kotlinx.serialization.json.*
import org.cuenora.shared.auth.AuthSession
import org.cuenora.shared.domain.Currency
import org.cuenora.shared.domain.OwnerId
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.network.ApiFailure
import org.cuenora.shared.onboarding.HttpOnboardingRepository
import org.cuenora.shared.repository.*
import org.cuenora.shared.serialization.ReportAmountSerializer

fun interface InsightsReader {
    suspend fun load(range: InsightsDateRange?): RepositoryResult<InsightsReport>
}

/** Read-only report client bound to a verified owner and one presentation lifetime. */
class AuthenticatedInsightsReader(
    private val session: AuthSession,
    private val api: ApiClient,
    private val today: (String) -> String?,
) : InsightsReader {
    private var owner: OwnerId? = null
    private var generation = 0
    private var retired = false

    fun clear() { owner = null; retired = true; generation++ }

    override suspend fun load(range: InsightsDateRange?): RepositoryResult<InsightsReport> = session.authorized { token ->
        val startedAt = generation
        if (retired) return@authorized denied()
        when (val profileResult = HttpOnboardingRepository(api, token).profile()) {
            is RepositoryResult.Failure -> if (startedAt != generation) denied() else profileResult
            is RepositoryResult.Success -> {
                val profile = profileResult.value
                val knownOwner = owner
                if (!profile.emailVerified || knownOwner != null && knownOwner != profile.ownerId) {
                    return@authorized RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
                }
                if (startedAt != generation) return@authorized denied()
                owner = profile.ownerId
                val requestedRange = range ?: run {
                    val zone = profile.timeZone?.takeIf(String::isNotBlank)
                        ?: return@authorized invalidResponse()
                    val profileDate = today(zone)?.let { runCatching { businessDate(it) }.getOrNull() }
                        ?: return@authorized invalidResponse()
                    InsightsDateRange.currentMonth(wireDate(profileDate))
                }
                val context = RepositoryContext(profile.ownerId, token)
                // Keep endpoint reads distinct: they do not share a server snapshot boundary.
                val flows = HttpInsightsReports.flows(api, context, requestedRange)
                if (startedAt != generation) return@authorized denied()
                if (flows is RepositoryResult.Failure) return@authorized flows
                val monthly = HttpInsightsReports.monthly(api, context, requestedRange)
                if (startedAt != generation) return@authorized denied()
                if (monthly is RepositoryResult.Failure) return@authorized monthly
                val distribution = HttpInsightsReports.distribution(api, context, requestedRange)
                if (startedAt != generation) return@authorized denied()
                if (distribution is RepositoryResult.Failure) return@authorized distribution
                RepositoryResult.Success(combine(
                    requestedRange,
                    (flows as RepositoryResult.Success).value,
                    (monthly as RepositoryResult.Success).value,
                    (distribution as RepositoryResult.Success).value,
                ))
            }
        }
    }

    private fun denied() = RepositoryResult.Failure(ApiFailure.Http(403, null, null, null))
    private fun invalidResponse() = RepositoryResult.Failure(ApiFailure.InvalidResponse(200, null))
}

private data class FlowRow(val currency: Currency, val income: String, val expenses: String, val net: String)
private data class MonthlyRow(val currency: Currency, val months: List<InsightsMonth>)
private data class DistributionRow(val currency: Currency, val expenses: String, val categories: List<InsightsCategory>)

private object HttpInsightsReports {
    suspend fun flows(api: ApiClient, context: RepositoryContext, range: InsightsDateRange) =
        mapResponse(api.request(queryPath("/v1/reports/flows", dates(range)), headers = context.headers()), 200) { root ->
            val obj = root.jsonObject.checkedRange(range)
            val rows = obj.array("currencies").map { value ->
                val row = value.jsonObject
                FlowRow(row.currency(), row.amount("income"), row.amount("expenses"), row.amount("net_flow", signed = true))
            }
            require(rows.map { it.currency } == currencies)
            rows
        }

    suspend fun monthly(api: ApiClient, context: RepositoryContext, range: InsightsDateRange) =
        mapResponse(api.request(queryPath("/v1/reports/monthly-flows", dates(range)), headers = context.headers()), 200) { root ->
            val obj = root.jsonObject.checkedRange(range)
            val expected = expectedMonths(range)
            val rows = obj.array("currencies").map { value ->
                val row = value.jsonObject
                val currency = row.currency()
                val months = row.array("months").map { entry ->
                    val month = entry.jsonObject
                    InsightsMonth(
                        month.string("month").also { require(it.matches(Regex("[0-9]{4}-(0[1-9]|1[0-2])"))) },
                        month.string("date_from").also(::businessDate),
                        month.string("date_to").also(::businessDate),
                        month.amount("income"), month.amount("expenses"), month.amount("net_flow", signed = true),
                    )
                }
                require(months.size == expected.size)
                require(months.map { Triple(it.month, it.dateFrom, it.dateTo) } ==
                    expected.map { Triple(it.first, it.second.first, it.second.second) })
                MonthlyRow(currency, months)
            }
            require(rows.map { it.currency } == currencies)
            rows
        }

    suspend fun distribution(api: ApiClient, context: RepositoryContext, range: InsightsDateRange) =
        mapResponse(api.request(queryPath("/v1/reports/expense-distribution", dates(range)), headers = context.headers()), 200) { root ->
            val obj = root.jsonObject.checkedRange(range)
            val rows = obj.array("currencies").map { value ->
                val row = value.jsonObject
                val currency = row.currency()
                val total = row.amount("expenses")
                val categories = row.array("categories").map { entry ->
                    val category = entry.jsonObject
                    val idText = category.nullableString("category_id")
                    val code = category.nullableString("code")
                    val name = category.nullableString("name")
                    val amount = category.amount("expenses")
                    val percentage = category.nullableString("percentage")?.also(::validateReportAmount)
                    require((idText == null) == (code == null && name == null))
                    require((idText != null) == (code != null && name != null))
                    if (total == "0.00") require(percentage == null) else require(percentage != null)
                    InsightsCategory(idText?.let(::categoryId), code, name, amount, percentage)
                }
                require(categories.size == 8 && categories.last().id == null && categories.dropLast(1).all { it.id != null })
                require(categories.mapNotNull { it.id }.distinct().size == 7)
                DistributionRow(currency, total, categories)
            }
            require(rows.map { it.currency } == currencies)
            rows
        }

    private fun dates(range: InsightsDateRange) = listOf("date_from" to range.dateFrom, "date_to" to range.dateTo)
}

private val currencies = listOf(Currency.SEK, Currency.EUR, Currency.USD)
private fun JsonObject.array(name: String): List<JsonElement> = getValue(name).jsonArray
private fun JsonObject.currency() = Currency.valueOf(string("currency"))
private fun JsonObject.amount(name: String, signed: Boolean = false): String =
    Json.decodeFromJsonElement(ReportAmountSerializer, JsonPrimitive(string(name))).also {
        if (!signed) require(!it.startsWith('-'))
    }

// The generated serializer validates the string token and exact two-decimal grammar without numeric conversion.
private fun validateReportAmount(value: String) {
    require(Regex("[0-9]+\\.[0-9]{2}").matches(value))
}

private fun JsonObject.checkedRange(range: InsightsDateRange): JsonObject {
    require(string("date_from") == range.dateFrom && string("date_to") == range.dateTo)
    return this
}

private fun combine(
    range: InsightsDateRange,
    flows: List<FlowRow>,
    monthly: List<MonthlyRow>,
    distribution: List<DistributionRow>,
): InsightsReport = InsightsReport(range, currencies.map { currency ->
    val flow = flows.single { it.currency == currency }
    val months = monthly.single { it.currency == currency }.months
    val distribution = distribution.single { it.currency == currency }
    InsightsCurrencyReport(currency, flow.income, flow.expenses, flow.net, months, distribution.categories, distribution.expenses)
})
