#!/usr/bin/env python3
"""Exercise the CI report validator through its public command-line interface."""

from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check_mobile_test_results.py"


class MobileTestResultsTest(unittest.TestCase):
    def run_report(self, xml):
        with tempfile.TemporaryDirectory(prefix="cuenora-junit-") as temp:
            directory = Path(temp) / "reports"
            if xml is not None:
                directory.mkdir()
                (directory / "TEST-shared.xml").write_text(xml)
            return subprocess.run(
                ["python3", str(SCRIPT), str(directory)],
                capture_output=True, text=True, check=False,
            )

    def test_executed_tests_pass(self):
        result = self.run_report('<testsuite tests="1" failures="0" errors="0" skipped="0">'
                                 '<testcase classname="SharedModuleTest" name="marker"/></testsuite>')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("1 tests passed", result.stdout)
        self.assertEqual(result.stderr, "")

    def test_missing_empty_and_malformed_reports_fail(self):
        for report in (None, '<testsuite tests="0"/>', '<broken', '<unrelated/>'):
            with self.subTest(report=report):
                result = self.run_report(report)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Invalid mobile test results", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_failure_error_or_skip_is_not_green(self):
        for kind in ("failure", "error", "skipped"):
            with self.subTest(kind=kind):
                # Child outcomes must not be hidden by incorrect summary attributes.
                report = ('<testsuite tests="1" failures="0" errors="0" skipped="0">'
                          f'<testcase name="bad"><{kind}/></testcase></testsuite>')
                result = self.run_report(report)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Invalid mobile test results", result.stderr)

    def test_counts_must_match_executed_cases(self):
        for attributes in ('tests="2"', 'tests="-1"', 'tests="no"',
                           'tests="1" failures="1"', 'tests="1" skipped="1"'):
            with self.subTest(attributes=attributes):
                result = self.run_report(f'<testsuite {attributes}><testcase name="one"/></testsuite>')
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Invalid mobile test results", result.stderr)


if __name__ == "__main__":
    unittest.main()
