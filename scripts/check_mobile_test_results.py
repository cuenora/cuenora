#!/usr/bin/env python3
"""Reject missing, empty, failed or skipped Gradle JUnit results."""

import argparse
from pathlib import Path
import sys
import xml.etree.ElementTree as ET


def validate(directory: Path) -> tuple[int, int]:
    reports = sorted(directory.glob("TEST-*.xml"))
    if not reports:
        raise ValueError("no JUnit XML reports found")
    total = 0
    for report in reports:
        suite = ET.parse(report).getroot()
        if suite.tag != "testsuite":
            raise ValueError("expected a Gradle testsuite")
        cases = suite.findall("testcase")
        count = int(suite.attrib["tests"])
        if count <= 0 or count != len(cases):
            raise ValueError("test count does not match executed cases")
        if any(int(suite.get(key, "0")) != 0 for key in ("failures", "errors", "skipped")):
            raise ValueError("suite contains failed or skipped tests")
        if any(case.find(tag) is not None for case in cases for tag in ("failure", "error", "skipped")):
            raise ValueError("test case failed or was skipped")
        total += count
    return total, len(reports)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, help="one Gradle target's JUnit XML directory")
    args = parser.parse_args()
    try:
        count, suites = validate(args.directory)
    except (OSError, ET.ParseError, ValueError, KeyError) as error:
        print(f"Invalid mobile test results: {error}", file=sys.stderr)
        return 1
    print(f"{count} tests passed ({suites} suites)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
