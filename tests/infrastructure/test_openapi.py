"""Standard-library structural guards for local component refs and savings routes.

These checks supplement, rather than replace, full YAML/OpenAPI validation.
"""
import re
import unittest
from pathlib import Path


CONTRACT = Path(__file__).resolve().parents[2] / "contracts" / "openapi.yaml"


class OpenApiStructureTest(unittest.TestCase):
    def test_all_local_component_references_have_definitions(self):
        source = CONTRACT.read_text()
        components = source.split("\ncomponents:\n", 1)[1]
        definitions = set()
        stack = [(-1, "components")]
        for line in components.splitlines():
            match = re.fullmatch(r"( *)([A-Za-z0-9_]+):(?: .*|)", line)
            if not match:
                continue
            indent = len(match[1])
            while stack[-1][0] >= indent:
                stack.pop()
            stack.append((indent, match[2]))
            definitions.add("#/" + "/".join(key for _, key in stack))
        references = set(re.findall(r"\$ref:\s*[\"']?(#/[^\s\"'}]+)", source))
        self.assertTrue(references)
        self.assertEqual(references - definitions, set())

    def test_account_reports_define_corrected_history_and_incomplete_planning(self):
        source = CONTRACT.read_text()
        for path, operation, schema in (("/v1/accounts/{id}/balance-history", "accountBalanceHistory", "AccountBalanceHistory"), ("/v1/reports/account-planning", "reportAccountPlanning", "AccountPlanning")):
            route = source.split(f"\n  {path}:\n", 1)[1]
            route = re.split(r"\n  \S", route, maxsplit=1)[0]
            self.assertIn(f"operationId: {operation}", route)
            self.assertIn("security: [{AccessBearer: []}]", route)
            self.assertIn(f"#/components/schemas/{schema}", route)
            self.assertIn("#/components/responses/AccountReportError", route)
        self.assertIn("history_basis: {type: string, const: current_corrected_ledger}", source)
        self.assertIn("maxItems: 366", source)
        schema = source.split("\n    AccountPlanning:\n", 1)[1]
        schema = re.split(r"\n    \S", schema, maxsplit=1)[0]
        for field in ("as_of", "through", "complete", "accounts", "unassigned", "current_balance", "pending_total", "projected_total", "planning_difference"):
            self.assertRegex(schema, rf"(?m)^ +{field}:")
        self.assertIn("Null when incomplete", schema)

    def test_monthly_flows_define_bounded_zero_filled_exact_calendar_series(self):
        source = CONTRACT.read_text()
        route = source.split("\n  /v1/reports/monthly-flows:\n", 1)[1]
        route = re.split(r"\n  \S", route, maxsplit=1)[0]
        self.assertIn("operationId: reportMonthlyFlows", route)
        self.assertIn("security: [{AccessBearer: []}]", route)
        for name in ("date_from", "date_to"):
            self.assertIn(f"name: {name}\n          in: query\n          required: true", route)
        self.assertIn("120", route)
        self.assertIn("#/components/schemas/MonthlyFlows", route)
        self.assertIn("#/components/responses/MonthlyFlowsError", route)
        schema = source.split("\n    MonthlyFlows:\n", 1)[1]
        schema = re.split(r"\n    \S", schema, maxsplit=1)[0]
        self.assertIn("maxItems: 120", schema)
        self.assertIn("required: [month, date_from, date_to, income, expenses, net_flow]", schema)
        for name in ("income", "expenses", "net_flow"):
            self.assertRegex(schema, rf"{name}: \{{type: string, pattern:")

    def test_expense_distribution_defines_exact_amounts_nullable_shares_and_dates(self):
        source = CONTRACT.read_text()
        route = source.split("\n  /v1/reports/expense-distribution:\n", 1)[1]
        route = re.split(r"\n  \S", route, maxsplit=1)[0]
        self.assertIn("operationId: reportExpenseDistribution", route)
        self.assertIn("security: [{AccessBearer: []}]", route)
        for name in ("date_from", "date_to"):
            self.assertIn(f"name: {name}\n          in: query\n          required: true", route)
        self.assertIn("#/components/schemas/ExpenseDistribution", route)
        self.assertIn("#/components/responses/ExpenseDistributionError", route)
        schema = source.split("\n    ExpenseDistribution:\n", 1)[1]
        schema = re.split(r"\n    \S", schema, maxsplit=1)[0]
        self.assertIn("required: [category_id, code, name, expenses, percentage]", schema)
        self.assertIn("Half-even percentage", schema)
        self.assertIn("type: [string, 'null']", schema)

    def test_flow_report_requires_dates_and_exact_currency_totals(self):
        source = CONTRACT.read_text()
        route = source.split("\n  /v1/reports/flows:\n", 1)[1]
        route = re.split(r"\n  \S", route, maxsplit=1)[0]
        self.assertIn("operationId: reportRealizedFlows", route)
        self.assertIn("security: [{AccessBearer: []}]", route)
        for name in ("date_from", "date_to"):
            self.assertIn(f"name: {name}\n          in: query\n          required: true", route)
        self.assertIn("#/components/schemas/RealizedFlows", route)
        self.assertIn("#/components/responses/FlowReportError", route)
        schema = source.split("\n    RealizedFlows:\n", 1)[1]
        schema = re.split(r"\n    \S", schema, maxsplit=1)[0]
        self.assertIn("required: [currency, income, expenses, net_flow]", schema)
        for name in ("income", "expenses", "net_flow"):
            self.assertRegex(schema, rf"{name}: \{{type: string, pattern:")

    def test_savings_forecast_defines_bounded_read_only_breakdown(self):
        source = CONTRACT.read_text()
        route = source.split("\n  /v1/accounts/{id}/savings-forecast:\n", 1)[1]
        route = re.split(r"\n  \S", route, maxsplit=1)[0]
        self.assertIn("operationId: forecastSavingsInterest", route)
        self.assertNotIn("\n    post:", route)
        for name in ("start_on", "end_on"):
            self.assertIn(f"name: {name}", route)
        self.assertIn("366", route)
        self.assertIn('#/components/schemas/SavingsForecast', route)
        self.assertIn('#/components/responses/SavingsForecastError', route)
        for field in ("posted_balance", "balance_as_of", "balance_assumption",
                      "includes_prior_accrual", "partial_first_period", "rate_segments",
                      "exact_accrual_minor_units", "projected_payout"):
            self.assertRegex(source, rf"(?m)^        {field}:")

    def test_savings_operations_define_success_and_shared_error_responses(self):
        source = CONTRACT.read_text()
        for stream in ("savings-terms", "savings-rates"):
            route = source.split(f"\n  /v1/accounts/{{id}}/{stream}:\n", 1)[1]
            route = re.split(r"\n  \S", route, maxsplit=1)[0]
            for method, status in (("get", "200"), ("post", "201")):
                operation = route.split(f"\n    {method}:\n", 1)[1]
                operation = re.split(r"\n    \S", operation, maxsplit=1)[0]
                self.assertIn(f'\n      responses:\n        "{status}":\n', operation)
                self.assertIn('\n        default:\n          $ref: "#/components/responses/SavingsConfigurationError"', operation)


if __name__ == "__main__":
    unittest.main()
