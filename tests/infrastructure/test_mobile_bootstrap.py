#!/usr/bin/env python3
"""Probe the real wrapper against loopback HTTP; fixture launch is not a Gradle build."""

import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import io
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import threading
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
WRAPPER = ROOT / "mobile/gradle/wrapper"


def properties():
    return dict(line.split("=", 1) for line in
                (WRAPPER / "gradle-wrapper.properties").read_text().splitlines()
                if line and not line.startswith("#"))


def java_tool(name):
    home = os.environ.get("JAVA_HOME")
    tool = str(Path(home) / "bin" / name) if home else shutil.which(name)
    if not tool or not Path(tool).is_file():
        raise RuntimeError("Bootstrap probes require a JDK (java and javac), not skipped tests.")
    return tool


class WrapperDownloadTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="cuenora-wrapper-")
        cls.addClassCleanup(cls.temp.cleanup)
        root = Path(cls.temp.name)
        source = root / "GradleMain.java"
        source.write_text('package org.gradle.launcher; public class GradleMain { '
                          'public static void main(String[] args) { '
                          'System.out.println("WRAPPER_FIXTURE_OK"); } }')
        subprocess.run([java_tool("javac"), "-d", str(root), str(source)],
                       capture_output=True, text=True, check=True)
        jar = io.BytesIO()
        with zipfile.ZipFile(jar, "w") as archive:
            archive.write(root / "org/gradle/launcher/GradleMain.class",
                          "org/gradle/launcher/GradleMain.class")
        distribution = io.BytesIO()
        with zipfile.ZipFile(distribution, "w") as archive:
            for directory in ("gradle-fixture/", "gradle-fixture/lib/", "gradle-fixture/bin/"):
                archive.writestr(directory, "")
            archive.writestr("gradle-fixture/bin/gradle", "#!/bin/sh\nexit 0\n")
            archive.writestr("gradle-fixture/lib/gradle-launcher-fixture.jar", jar.getvalue())
        cls.distribution = distribution.getvalue()

    def run_wrapper(self, failures, wrong_checksum=False):
        distribution = self.distribution
        requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append(self.path)
                if len(requests) <= failures:
                    self.send_error(504, "Gateway Timeout")
                else:
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(distribution)))
                    self.end_headers()
                    self.wfile.write(distribution)

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            with tempfile.TemporaryDirectory(prefix="cuenora-wrapper-probe-") as temp:
                root = Path(temp)
                shutil.copy2(WRAPPER / "gradle-wrapper.jar", root / "gradle-wrapper.jar")
                config = properties()
                config["distributionUrl"] = f"http://127.0.0.1:{server.server_port}/gradle.zip"
                config["distributionSha256Sum"] = ("0" * 64 if wrong_checksum else
                                                    hashlib.sha256(distribution).hexdigest())
                (root / "gradle-wrapper.properties").write_text(
                    "\n".join(f"{key}={value}" for key, value in config.items()) + "\n")
                result = subprocess.run(
                    [java_tool("java"), "-jar", str(root / "gradle-wrapper.jar"), "--version"],
                    cwd=root, env={**os.environ, "GRADLE_USER_HOME": str(root / "cache")},
                    text=True, capture_output=True, timeout=40, check=False,
                )
                cache_accepted = list((root / "cache").rglob("*.ok"))
                return result, requests, cache_accepted
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=5)

    def test_policy_preserves_version_checksum_and_bounded_retries(self):
        config = properties()
        self.assertEqual(config["distributionUrl"],
                         r"https\://services.gradle.org/distributions/gradle-9.5.0-bin.zip")
        self.assertEqual(config["distributionSha256Sum"],
                         "553c78f50dafcd54d65b9a444649057857469edf836431389695608536d6b746")
        self.assertEqual(config["validateDistributionUrl"], "true")
        self.assertEqual(config["networkTimeout"], "60000")
        self.assertEqual(config.get("retries"), "3")
        self.assertEqual(config.get("retryBackOffMs"), "1000")

    def test_first_attempt_success_still_launches_verified_fixture(self):
        result, requests, accepted = self.run_wrapper(0)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("WRAPPER_FIXTURE_OK", result.stdout)
        self.assertEqual(len(requests), 1)
        self.assertEqual(len(accepted), 1)

    def test_504_recovers_without_rerunning_a_build(self):
        result, requests, accepted = self.run_wrapper(1)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("WRAPPER_FIXTURE_OK", result.stdout)
        self.assertEqual(len(requests), 2)
        self.assertEqual(len(accepted), 1)

    def test_persistent_504_fails_after_four_attempts(self):
        result, requests, accepted = self.run_wrapper(100)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(requests), 4)
        self.assertNotIn("WRAPPER_FIXTURE_OK", result.stdout)
        self.assertEqual(accepted, [])

    def test_checksum_mismatch_never_launches_or_accepts_distribution(self):
        result, _, accepted = self.run_wrapper(0, wrong_checksum=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", (result.stdout + result.stderr).lower())
        self.assertNotIn("WRAPPER_FIXTURE_OK", result.stdout)
        self.assertEqual(accepted, [])


class RunnerJavaTest(unittest.TestCase):
    def run_setup(self, arch, major="25", missing=False):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        step = workflow.split("      - name: Select runner-provided Java 25\n", 1)[1]
        step = step.split("\n      - name:", 1)[0]
        script = textwrap.dedent(step.split("        run: |\n", 1)[1])
        with tempfile.TemporaryDirectory(prefix="cuenora-java-probe-") as temp:
            root = Path(temp)
            home = root / "jdk with spaces"
            java = home / "bin/java"
            java.parent.mkdir(parents=True)
            java.write_text("#!/bin/sh\n" +
                            f"echo '    java.specification.version = {major}' >&2\n")
            java.chmod(0o700)
            env_file, path_file = root / "env", root / "path"
            env_file.touch()
            path_file.touch()
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith("JAVA_HOME_25_")}
            env.update(RUNNER_ARCH=arch, GITHUB_ENV=str(env_file), GITHUB_PATH=str(path_file))
            variable = "JAVA_HOME_25_X64" if arch == "X64" else "JAVA_HOME_25_arm64"
            if not missing:
                env[variable] = str(home)
            result = subprocess.run(["bash", "-e", "-o", "pipefail", "-c", script],
                                    env=env, text=True, capture_output=True, check=False)
            return result, env_file.read_text(), path_file.read_text(), str(home)

    def test_both_runner_architectures_publish_validated_java_home(self):
        for arch in ("X64", "ARM64"):
            with self.subTest(arch=arch):
                result, env, path, home = self.run_setup(arch)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(env, f"JAVA_HOME={home}\n")
                self.assertEqual(path, f"{home}/bin\n")

    def test_missing_or_wrong_major_fails_without_environment_changes(self):
        for arch, major, missing in (("ARM64", "25", True), ("X64", "26", False)):
            with self.subTest(arch=arch, major=major, missing=missing):
                result, env, path, _ = self.run_setup(arch, major, missing)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual((env, path), ("", ""))


if __name__ == "__main__":
    unittest.main()
