package org.cuenora.shared.planning

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import org.cuenora.shared.domain.*
import kotlin.test.*

class PlanningModelsTest {
    @Test fun exactDecimalAggregatesPastLongWithoutNarrowing() {
        assertEquals("200000000000000000000000000000000000.01", ExactDecimal.add("99999999999999999999999999999999999.99", "100000000000000000000000000000000000.02"))
        assertEquals("-1.25", ExactDecimal.subtract("0.00", "1.25"))
        assertEquals("0.00", ExactDecimal.add("-3.10", "3.10"))
    }

    @Test fun rejectsIncompleteDifferenceAndCurrencyOrderingContradictions() {
        val incomplete = sample.replace("\"complete\":true,\"accounts\"", "\"complete\":false,\"accounts\"")
            .replace("\"complete\":true}]}\n", "\"complete\":false}]}\n")
        assertFails { parsePlanning(Json.parseToJsonElement(incomplete).jsonObject) }
        val changedCurrency = sample.replace("\"currency\":\"EUR\",\"pending_total\":\"0.00\"", "\"currency\":\"USD\",\"pending_total\":\"0.00\"")
        assertFails { parsePlanning(Json.parseToJsonElement(changedCurrency).jsonObject) }
    }

    @Test fun exactCompleteDifferenceAndArchivedAccountAreRetained() {
        val data = parsePlanning(Json.parseToJsonElement(sample).jsonObject)
        assertEquals("-500.00", data.accounts.single().planningDifference)
        assertEquals(AccountStatus.ARCHIVED, data.accounts.single().status)
        assertEquals(listOf(Currency.SEK, Currency.EUR, Currency.USD), data.unassigned.map { it.currency })
    }

    private val sample = """{"as_of":"2026-11-10T12:00:00Z","through":"2026-11-30","complete":true,"accounts":[{"account_id":"00000000-0000-0000-0000-000000000001","name":"Old","account_type":"savings","status":"archived","currency":"SEK","current_balance":"300.00","pending_total":"200.00","projected_total":"600.00","planning_difference":"-500.00","complete":true}],"unassigned":[{"currency":"SEK","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"EUR","pending_total":"0.00","projected_total":"0.00","complete":true},{"currency":"USD","pending_total":"0.00","projected_total":"0.00","complete":true}]}"""
}
