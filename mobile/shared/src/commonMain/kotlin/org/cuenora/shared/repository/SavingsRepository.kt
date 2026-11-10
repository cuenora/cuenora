package org.cuenora.shared.repository

import io.ktor.http.HttpMethod
import kotlinx.serialization.json.*
import org.cuenora.shared.domain.*
import org.cuenora.shared.network.ApiClient
import org.cuenora.shared.validation.*

/** Immutable date-slot identity: retries retain account, stream and effective date. */
data class SavingsTermsCommand(val effectiveOn: BusinessDate, val payoutFrequency: String, val capitalize: Boolean) {
    init { require(payoutFrequency in setOf("daily", "monthly", "quarterly", "annual")) }
}
data class SavingsRateCommand(val effectiveOn: BusinessDate, val nominalAnnualPercent: String) {
    init { require(validateSavingsRateForm(SavingsRateForm(wireDate(effectiveOn), nominalAnnualPercent)).issues.isEmpty()) }
}
data class SavingsTermsVersion(val id: String, val terms: SavingsTermsCommand)
data class SavingsRateVersion(val id: String, val rate: SavingsRateCommand)
data class SavingsHistoryQuery(val limit: Int = 50, val after: BusinessDate? = null) {
    init { require(limit in 1..100) }
}
data class SavingsRateSegment(val start: BusinessDate, val end: BusinessDate, val effectiveOn: BusinessDate,
    val nominalAnnualPercent: String, val daysInYear: Int)
data class SavingsEstimatePeriod(val start: BusinessDate, val end: BusinessDate, val accrual: ExactAccrual,
    /** Arbitrary-precision decimal currency string, intentionally not recorded Money. */
    val projectedPayout: String?)
data class SavingsEstimate(val accountId: AccountId, val currency: Currency, val start: BusinessDate,
    val end: BusinessDate, val balanceAsOf: BusinessDate, val postedBalance: Money,
    val balanceAssumption: String, val includesPriorAccrual: Boolean, val partialFirstPeriod: Boolean,
    val terms: SavingsTermsCommand, val rates: List<SavingsRateSegment>, val periods: List<SavingsEstimatePeriod>)

interface SavingsRepository {
    suspend fun terms(account: AccountId, query: SavingsHistoryQuery = SavingsHistoryQuery()): RepositoryResult<Page<SavingsTermsVersion>>
    suspend fun rates(account: AccountId, query: SavingsHistoryQuery = SavingsHistoryQuery()): RepositoryResult<Page<SavingsRateVersion>>
    suspend fun appendTerms(account: AccountId, command: SavingsTermsCommand): RepositoryResult<SavingsTermsVersion>
    suspend fun appendRate(account: AccountId, command: SavingsRateCommand): RepositoryResult<SavingsRateVersion>
    suspend fun forecast(account: AccountId, start: BusinessDate, end: BusinessDate): RepositoryResult<SavingsEstimate>
}

/** HTTP mapping only; no calculation, implicit date, replay or session lifecycle. */
class HttpSavingsRepository(private val api: ApiClient, private val context: RepositoryContext) : SavingsRepository {
    override suspend fun terms(account: AccountId, query: SavingsHistoryQuery) = history(account, "savings-terms", query) {
        SavingsTermsVersion(versionId(it), decodeTerms(it))
    }
    override suspend fun rates(account: AccountId, query: SavingsHistoryQuery) = history(account, "savings-rates", query) {
        SavingsRateVersion(versionId(it), decodeRate(it))
    }
    private suspend fun <T> history(account: AccountId, stream: String, query: SavingsHistoryQuery,
        decode: (JsonObject) -> T): RepositoryResult<Page<T>> = mapResponse(api.request(queryPath(path(account, stream),
            listOf("limit" to query.limit.toString(), "cursor" to query.after?.let(::wireDate))), headers = context.headers()), 200) {
        val root = it.jsonObject
        val versions = root.getValue("versions").jsonArray
        require(versions.size <= query.limit)
        val dates = versions.map { version -> businessDate(version.jsonObject.string("effective_on")) }
        require(dates.zipWithNext().all { (a, b) -> a < b } && dates.all { date -> query.after?.let { date > it } != false })
        val cursor = root.nullableString("next_cursor")
        if (cursor != null) require(dates.isNotEmpty() && businessDate(cursor) == dates.last())
        Page(versions.map { version -> decode(version.jsonObject) }, cursor)
    }
    override suspend fun appendTerms(account: AccountId, command: SavingsTermsCommand): RepositoryResult<SavingsTermsVersion> =
        mapResponse(api.request(path(account, "savings-terms"), HttpMethod.Post, encodeTerms(command), context.headers()), 201) {
            val root = it.jsonObject
            SavingsTermsVersion(versionId(root), decodeTerms(root)).also { version -> require(version.terms == command) }
        }
    override suspend fun appendRate(account: AccountId, command: SavingsRateCommand): RepositoryResult<SavingsRateVersion> =
        mapResponse(api.request(path(account, "savings-rates"), HttpMethod.Post, buildJsonObject {
            put("effective_on", wireDate(command.effectiveOn)); put("nominal_annual_percent", command.nominalAnnualPercent)
        }, context.headers()), 201) {
            val root = it.jsonObject
            SavingsRateVersion(versionId(root), decodeRate(root)).also { version ->
                require(version.rate.effectiveOn == command.effectiveOn && normalizedRate(version.rate.nominalAnnualPercent) == normalizedRate(command.nominalAnnualPercent))
            }
        }
    override suspend fun forecast(account: AccountId, start: BusinessDate, end: BusinessDate): RepositoryResult<SavingsEstimate> {
        require(start <= end)
        return mapResponse(api.request(queryPath(path(account, "savings-forecast"),
            listOf("start_on" to wireDate(start), "end_on" to wireDate(end))), headers = context.headers()), 200) {
            val root = it.jsonObject
            require(accountId(root.string("account_id")) == account && businessDate(root.string("start_on")) == start && businessDate(root.string("end_on")) == end)
            require(root.string("balance_assumption") == "constant_posted_snapshot" && !root.boolean("includes_prior_accrual"))
            val currency = Currency.valueOf(root.string("currency"))
            val rates = root.getValue("rate_segments").jsonArray.map { element ->
                val row = element.jsonObject
                val a = businessDate(row.string("start_on")); val b = businessDate(row.string("end_on"))
                val effective = businessDate(row.string("effective_on"))
                val rate = SavingsRateCommand(effective, row.string("nominal_annual_percent"))
                val divisor = row.getValue("days_in_year").jsonPrimitive.also { require(!it.isString) }.int
                require(a <= b && a >= start && b <= end && effective <= a && divisor in setOf(365, 366))
                SavingsRateSegment(a, b, effective, rate.nominalAnnualPercent, divisor)
            }
            val periods = root.getValue("periods").jsonArray.map { element ->
                val row = element.jsonObject
                val a = businessDate(row.string("start_on")); val b = businessDate(row.string("end_on"))
                require(a <= b && a >= start && b <= end)
                val exact = row.getValue("exact_accrual_minor_units").jsonObject
                val payout = row.nullableString("projected_payout")
                require(payout == null || Regex("[0-9]+\\.[0-9]{2}").matches(payout))
                SavingsEstimatePeriod(a, b, ExactAccrual(exact.string("numerator"), exact.string("denominator")), payout)
            }
            require(rates.isNotEmpty() && periods.isNotEmpty() && rates.first().start == start && rates.last().end == end &&
                periods.first().start == start && periods.last().end == end)
            require(rates.zipWithNext().all { (a, b) -> nextDate(a.end) == b.start } && periods.zipWithNext().all { (a, b) -> nextDate(a.end) == b.start })
            SavingsEstimate(account, currency, start, end, businessDate(root.string("balance_as_of")),
                readMoney(root.getValue("posted_balance"), currency), "constant_posted_snapshot", false,
                root.boolean("partial_first_period"), decodeTerms(root.getValue("terms").jsonObject), rates, periods)
        }
    }
    private fun path(account: AccountId, suffix: String) = "/v1/accounts/${wireId(account)}/$suffix"
}

internal fun encodeTerms(command: SavingsTermsCommand) = buildJsonObject {
    put("effective_on", wireDate(command.effectiveOn)); put("payout_frequency", command.payoutFrequency)
    put("capitalize", command.capitalize); put("rate_basis", "nominal_annual"); put("day_count", "actual_calendar_year")
    put("balance_timing", "daily_close"); put("rounding", "period_end_half_even")
}
private fun decodeTerms(row: JsonObject): SavingsTermsCommand {
    require(row.string("rate_basis") == "nominal_annual" && row.string("day_count") == "actual_calendar_year" &&
        row.string("balance_timing") == "daily_close" && row.string("rounding") == "period_end_half_even")
    return SavingsTermsCommand(businessDate(row.string("effective_on")), row.string("payout_frequency"), row.boolean("capitalize"))
}
private fun decodeRate(row: JsonObject) = SavingsRateCommand(businessDate(row.string("effective_on")), row.string("nominal_annual_percent"))
private fun versionId(row: JsonObject) = row.string("id").also { uuidBits(it) }
private fun JsonObject.boolean(name: String) = getValue(name).jsonPrimitive.also { require(!it.isString && it.content in setOf("true", "false")) }.boolean
private fun normalizedRate(rate: String): String {
    val parts = rate.split('.')
    return parts[0].trimStart('0').ifEmpty { "0" } + "." + parts.getOrElse(1) { "" }.trimEnd('0')
}
private fun nextDate(date: BusinessDate): BusinessDate? {
    if (date.day < 28) return BusinessDate(date.year, date.month, date.day + 1)
    return try { BusinessDate(date.year, date.month, date.day + 1) } catch (_: IllegalArgumentException) {
        if (date.month < 12) BusinessDate(date.year, date.month + 1, 1)
        else if (date.year < 9999) BusinessDate(date.year + 1, 1, 1) else null
    }
}
