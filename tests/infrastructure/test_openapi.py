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
