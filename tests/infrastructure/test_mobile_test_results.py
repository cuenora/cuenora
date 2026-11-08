#!/usr/bin/env python3
"""Exercise the mobile CI SDK step and public report-validation command."""

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check_mobile_test_results.py"


class MobileSdkPackagesTest(unittest.TestCase):
    def test_workflow_requests_published_sdk_package_ids(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        step = workflow.split("      - name: Ensure Android SDK packages\n", 1)[1]
        step = step.split("\n      - name:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        # IDs checked against Google's repository2-3.xml on 2026-11-08.
        packages = ("platforms;android-36", "platforms;android-37.0",
                    "build-tools;36.0.0", "build-tools;37.0.0")
        with tempfile.TemporaryDirectory(prefix="cuenora-sdk regression-") as temp:
            sdk = Path(temp) / "sdk"
            manager = sdk / "cmdline-tools/latest/bin/sdkmanager"
            manager.parent.mkdir(parents=True)
            manager.write_text("#!/usr/bin/env python3\nimport sys\n"
                               f"available = {packages!r}\n"
                               "for package in sys.argv[1:]:\n"
                               "    if package not in available:\n"
                               "        sys.exit('Failed to find package: ' + package)\n"
                               "print('\\n'.join(sys.argv[1:]))\n")
            manager.chmod(0o700)
            result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script],
                                    env={**os.environ, "ANDROID_HOME": str(sdk)},
                                    text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(), list(packages))
            self.assertEqual(result.stderr, "")


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
