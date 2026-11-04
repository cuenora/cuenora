#!/usr/bin/env python3
"""Exercise the publication shell without credentials, network or Docker writes."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github/workflows/docker-alpha.yml"


def publication_script() -> str:
    text = WORKFLOW.read_text()
    step = text.split("      - name: Push architecture images and assemble manifest\n", 1)[1]
    return textwrap.dedent(step.split("        run: |\n", 1)[1])


FAKE_DOCKER = '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
with (Path(os.environ['RUNNER_TEMP']) / 'calls.jsonl').open('a') as log:
    log.write(json.dumps(args) + '\\n')
if args[0] == 'push' and os.environ.get('FAIL_ARCH') and args[-1].endswith(os.environ['FAIL_ARCH']):
    sys.exit(1)
if args[:2] == ['image', 'inspect']:
    arch = 'amd64' if args[-1].endswith('amd64') else 'arm64'
    if args[3] == '{{.Architecture}}':
        print(arch)
    else:
        digit = 'a' if arch == 'amd64' else 'b'
        print('docker.io/carfdev/cuenora@sha256:' + digit * 64)
if args[:2] == ['manifest', 'inspect']:
    print(json.dumps({'manifests': [{'platform': {'os': 'linux', 'architecture': a}} for a in ['amd64', 'arm64']]}))
'''


class DockerAlphaWorkflowTests(unittest.TestCase):
    def execute(self, fail_arch: str = "") -> tuple[int, list[list[str]], str]:
        with tempfile.TemporaryDirectory(prefix="cuenora-alpha-") as temp:
            root = Path(temp)
            docker = root / "docker"
            docker.write_text(FAKE_DOCKER)
            docker.chmod(0o700)
            env = os.environ.copy()
            env.update(
                PATH=temp + os.pathsep + env["PATH"],
                RUNNER_TEMP=temp,
                GITHUB_STEP_SUMMARY=str(root / "summary"),
                IMAGE="docker.io/carfdev/cuenora",
                VERSION="0.1.0-alpha.1",
                RUN_ID="123",
                RUN_ATTEMPT="1",
                GITHUB_SHA="c" * 40,
                FAIL_ARCH=fail_arch,
            )
            result = subprocess.run(
                ["bash", "-e", "-o", "pipefail", "-c", publication_script()],
                env=env, text=True, capture_output=True, check=False,
            )
            calls = [json.loads(line) for line in (root / "calls.jsonl").read_text().splitlines()]
            summary = (root / "summary").read_text() if (root / "summary").exists() else ""
            return result.returncode, calls, summary

    def test_both_uploads_precede_digest_bound_manifest(self) -> None:
        code, calls, summary = self.execute()
        self.assertEqual(code, 0)
        pushes = [i for i, call in enumerate(calls) if call[0] == "push"]
        creates = [(i, call) for i, call in enumerate(calls) if call[:2] == ["manifest", "create"]]
        self.assertEqual(len(pushes), 2)
        self.assertEqual(len(creates), 1)
        index, command = creates[0]
        self.assertGreater(index, max(pushes))
        self.assertEqual(command[2], "docker.io/carfdev/cuenora:0.1.0-alpha.1")
        self.assertEqual(len(command[3:]), 2)
        self.assertTrue(all("@sha256:" in ref for ref in command[3:]))
        self.assertIn("Platforms: linux/amd64, linux/arm64", summary)
        self.assertNotIn(":latest", str(calls))

    def test_failed_upload_never_creates_final_tag(self) -> None:
        for arch in ("amd64", "arm64"):
            with self.subTest(arch=arch):
                code, calls, summary = self.execute(arch)
                self.assertNotEqual(code, 0)
                self.assertFalse(any(call[:2] == ["manifest", "create"] for call in calls))
                self.assertEqual(summary, "")


if __name__ == "__main__":
    unittest.main()
