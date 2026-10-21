"""Exercise local PostgreSQL using disposable Compose projects and synthetic data."""

import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
COMPOSE = ROOT / "infra" / "compose.yaml"


class PostgresComposeTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="cuenora-postgres-")
        self.addCleanup(self.scratch.cleanup)
        self.projects = []
        self.addCleanup(self.remove_projects)
        self.env = dict(os.environ)
        # Do not load a developer's .env or default Compose project settings.
        self.env.pop("COMPOSE_PROJECT_NAME", None)
        self.env.pop("COMPOSE_PROFILES", None)
        self.env["CUENORA_POSTGRES_PASSWORD"] = secrets.token_urlsafe(32)
        self.env["CUENORA_POSTGRES_PORT"] = "0"

    def compose(self, project, *args, env=None):
        result = subprocess.run(
            ["docker", "compose", "--env-file", os.devnull, "-f", str(COMPOSE),
             "-p", project, *args],
            cwd=self.scratch.name, env=self.env if env is None else env,
            capture_output=True, text=True, timeout=180,
        )
        # Do not print captured output: configuration errors may contain secrets.
        return result

    def succeeds(self, project, *args, env=None):
        result = self.compose(project, *args, env=env)
        self.assertEqual(result.returncode, 0, f"Compose {args[0]} failed")
        return result.stdout.strip()

    def new_project(self):
        project = "cuenora-test-" + secrets.token_hex(6)
        self.projects.append(project)
        self.succeeds(project, "up", "-d", "--wait", "--wait-timeout", "90")
        return project

    def sql(self, project, query):
        # Pass the password via environment, never command arguments or output.
        env = dict(self.env, PGPASSWORD=self.env["CUENORA_POSTGRES_PASSWORD"])
        return self.succeeds(
            project, "exec", "-T", "-e", "PGPASSWORD", "postgres", "psql",
            "-h", "127.0.0.1", "-U", "cuenora", "-d", "cuenora",
            "-v", "ON_ERROR_STOP=1", "-Atc", query, env=env,
        )

    def remove_projects(self):
        for project in self.projects:
            result = self.compose(project, "down", "--volumes")
            self.assertEqual(result.returncode, 0, "Temporary project cleanup failed")
            for kind in ("container", "volume", "network"):
                result = subprocess.run(
                    ["docker", kind, "ls", "-q", "--filter",
                     f"label=com.docker.compose.project={project}"],
                    env=self.env, capture_output=True, text=True, timeout=30,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout.strip(), "", f"Leaked test {kind}")

    def test_missing_password(self):
        env = dict(self.env)
        env.pop("CUENORA_POSTGRES_PASSWORD")
        result = self.compose("cuenora-test-missing-password", "config", "--quiet", env=env)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CUENORA_POSTGRES_PASSWORD", result.stderr)

    def test_lifecycle(self):
        self.assertTrue(COMPOSE.is_file(), "Compose manifest is missing")
        first = self.new_project()
        self.assertEqual(self.sql(first, "SELECT current_database();"), "cuenora")
        container = self.succeeds(first, "ps", "-q", "postgres")
        result = subprocess.run(
            ["docker", "inspect", container], env=self.env,
            capture_output=True, text=True, check=True, timeout=30,
        )
        info = json.loads(result.stdout)[0]
        self.assertEqual(info["State"]["Health"]["Status"], "healthy")
        bindings = info["NetworkSettings"]["Ports"]["5432/tcp"]
        self.assertTrue(bindings)
        self.assertTrue(all(binding["HostIp"] == "127.0.0.1" for binding in bindings))
        wrong = dict(self.env, PGPASSWORD=secrets.token_urlsafe(32))
        denied = self.compose(
            first, "exec", "-T", "-e", "PGPASSWORD", "postgres", "psql",
            "-h", "127.0.0.1", "-U", "cuenora", "-d", "cuenora", "-Atc", "SELECT 1;",
            env=wrong,
        )
        self.assertNotEqual(denied.returncode, 0)
        self.assertIn("password authentication failed", denied.stderr)
        self.sql(first, "CREATE TABLE smoke_probe (value integer); INSERT INTO smoke_probe VALUES (42);")
        self.succeeds(first, "down")
        self.succeeds(first, "up", "-d", "--wait", "--wait-timeout", "90")
        self.assertEqual(self.sql(first, "SELECT value FROM smoke_probe;"), "42")
        second = self.new_project()
        self.assertEqual(self.sql(second, "SELECT to_regclass('public.smoke_probe') IS NULL;"), "t")
        self.assertNotEqual(
            self.succeeds(first, "port", "postgres", "5432"),
            self.succeeds(second, "port", "postgres", "5432"),
        )


if __name__ == "__main__":
    unittest.main()
