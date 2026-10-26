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
MIGRATIONS = ROOT / "backend" / "migrations"


def identity_id(value):
    """Deterministic identifiers for synthetic database fixtures only."""
    return f"00000000-0000-0000-0000-{value:012d}"


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

    def identity_result(self, project, query):
        env = dict(self.env, PGPASSWORD=self.env["CUENORA_POSTGRES_PASSWORD"])
        return self.compose(
            project, "exec", "-T", "-e", "PGPASSWORD", "postgres", "psql",
            "-X", "-h", "127.0.0.1", "-U", "cuenora", "-d", "cuenora",
            "-v", "ON_ERROR_STOP=1", "-v", "VERBOSITY=sqlstate",
            "--single-transaction", "-Atc", query, env=env,
        )

    def identity_project(self):
        project = self.new_project()
        for migration in sorted(MIGRATIONS.glob("*.sql")):
            result = self.identity_result(project, migration.read_text())
            self.assertEqual(result.returncode, 0, "Migration application failed")
        return project

    def seed_users(self, project):
        self.sql(project, f"""
            INSERT INTO users (id, email_original, email_address) VALUES
                ('{identity_id(1)}', 'Ana@example.test', 'Ana@example.test'),
                ('{identity_id(2)}', 'Ben@example.test', 'Ben@example.test');
        """)

    def identity_snapshot(self, project):
        return self.sql(project, """
            SELECT md5(jsonb_build_array(
                (SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM users u),
                (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY user_id), '[]') FROM password_credentials c),
                (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY id), '[]') FROM user_sessions s),
                (SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]') FROM session_refresh_tokens t)
            )::text);
        """)

    def access_snapshot(self, project):
        return self.identity_snapshot(project), self.sql(project, """
            SELECT md5(coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]')::text)
            FROM session_access_tokens t;
        """)

    def reject_access_write(self, project, query, sqlstate):
        before = self.access_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid access-token write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.access_snapshot(project), before)

    def reject_identity_write(self, project, query, sqlstate):
        before = self.identity_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid identity write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.identity_snapshot(project), before)

    def test_all_migrations_reproduce_schema_on_fresh_instances(self):
        self.assertTrue(list(MIGRATIONS.glob("*.sql")), "No migrations found")
        expected_tables = {
            "users", "password_credentials", "user_sessions", "session_refresh_tokens",
            "session_access_tokens",
            "accounts", "categories", "transactions", "account_movements", "transaction_audit",
        }
        schemas = []
        seeds = []
        for _ in range(2):
            project = self.identity_project()
            tables = self.sql(project, """
                SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename;
            """)
            self.assertEqual(set(tables.splitlines()), expected_tables)
            self.assertEqual(self.sql(project, """
                SELECT count(*) FROM pg_constraint c
                JOIN pg_namespace n ON n.oid = c.connamespace
                WHERE n.nspname = 'public' AND NOT c.convalidated;
            """), "0")
            self.assertEqual(self.sql(project, """
                SELECT count(*) FROM pg_constraint WHERE conname IN (
                    'accounts_owner_currency', 'accounts_owner_type',
                    'transactions_account_currency', 'transactions_destination_currency',
                    'transactions_income_account_type', 'account_movements_account_currency'
                );
            """), "6")
            env = dict(self.env, PGPASSWORD=self.env["CUENORA_POSTGRES_PASSWORD"])
            dump = self.succeeds(
                project, "exec", "-T", "-e", "PGPASSWORD", "postgres", "pg_dump",
                "-h", "127.0.0.1", "-U", "cuenora", "-d", "cuenora",
                "--schema-only", "--no-owner", "--no-privileges", env=env,
            )
            # PostgreSQL generates a different psql safety token for each dump.
            schemas.append("\n".join(
                line for line in dump.splitlines()
                if not line.startswith(("\\restrict ", "\\unrestrict "))
            ))
            seeds.append(self.sql(project, """
                SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM categories c;
            """))
            self.assertNotEqual(seeds[-1], "", "Built-in category seed is missing")
            for table in expected_tables - {"categories"}:
                self.assertEqual(self.sql(project, f"SELECT count(*) FROM {table};"), "0")
        self.assertEqual(schemas[0], schemas[1], "Fresh schema dumps differ")
        self.assertEqual(seeds[0], seeds[1], "Fresh category seeds differ")

    def test_registration_store_against_disposable_postgres(self):
        self.registration_target("registration_store")

    def test_registration_http_against_disposable_postgres(self):
        self.registration_target("registration_http")

    def test_login_store_against_disposable_postgres(self):
        self.registration_target("login_store")

    def test_login_http_against_disposable_postgres(self):
        self.registration_target("login_http", expected_tests=5)

    def registration_target(self, target, expected_tests=4):
        project = self.identity_project()
        address = self.succeeds(project, "port", "postgres", "5432")
        host, port = address.rsplit(":", 1)
        self.assertEqual(host, "127.0.0.1", "Test database must be local")
        env = dict(self.env, CUENORA_TEST_DISPOSABLE="1", CUENORA_TEST_PGPORT=port,
                   CUENORA_TEST_PGPASSWORD=self.env["CUENORA_POSTGRES_PASSWORD"])
        result = subprocess.run(
            ["rustup", "run", "1.99.1", "cargo", "test", "-p", "cuenora-api",
             "--test", target, "--locked", "--", "--ignored", "--test-threads=1"],
            cwd=ROOT / "backend", env=env, capture_output=True, text=True, timeout=240,
        )
        # Report Cargo diagnostics, never connection secrets.
        safe_output = (result.stdout + result.stderr).replace(env["CUENORA_TEST_PGPASSWORD"], "[redacted]")
        self.assertEqual(result.returncode, 0, safe_output)
        self.assertIn(f"{expected_tests} passed", result.stdout)

    def test_identity_migrations_create_reproducible_schema(self):
        project = self.identity_project()
        for table in ["users", "password_credentials", "user_sessions", "session_refresh_tokens"]:
            self.assertEqual(self.sql(project, f"SELECT to_regclass('{table}') IS NOT NULL;"), "t")
        self.seed_users(project)
        self.assertEqual(self.sql(project, "SELECT count(*) FROM users;"), "2")
        self.assertEqual(self.sql(project, "SELECT pg_typeof(id) FROM users LIMIT 1;"), "uuid")
        first = sorted(MIGRATIONS.glob("*.sql"))[0]
        self.reject_identity_write(project, first.read_text(), "42P07")

    def test_identity_email_keys_preserve_original_and_reject_case_duplicates(self):
        project = self.identity_project()
        self.sql(project, f"""
            INSERT INTO users (id, email_original, email_address)
            VALUES ('{identity_id(1)}', 'AnA@mañana.example', 'AnA@xn--maana-pta.example');
        """)
        self.assertEqual(self.sql(project, "SELECT email_original || '|' || email_key FROM users;"),
                         "AnA@mañana.example|ana@xn--maana-pta.example")
        self.reject_identity_write(project, f"""
            INSERT INTO users (id, email_original, email_address)
            VALUES ('{identity_id(2)}', 'ana@xn--maana-pta.example', 'ana@xn--maana-pta.example');
        """, "23505")
        for value, address in [(3, 'a.na@example.test'), (4, 'ana+tag@example.test')]:
            self.sql(project, f"INSERT INTO users (id, email_original, email_address) VALUES ('{identity_id(value)}', '{address}', '{address}');")
        self.assertEqual(self.sql(project, "SELECT count(*) FROM users;"), "3")
        for original, address in [
            ('josé@example.test', 'josé@example.test'),
            ('ana@mañana.example', 'ana@mañana.example'),
            ('ana@example.test', 'different@example.test'),
            ('', ''),
        ]:
            self.reject_identity_write(project, f"INSERT INTO users (email_original, email_address) VALUES ('{original}', '{address}');", "23514")
        self.reject_identity_write(project, """
            INSERT INTO users (email_original, email_address, email_key)
            VALUES ('new@example.test', 'new@example.test', 'override');
        """, "428C9")

    def test_identity_credentials_and_unconfirmed_profile_constraints(self):
        project = self.identity_project()
        self.seed_users(project)
        self.assertEqual(self.sql(project, "SELECT bool_and(profile_time_zone IS NULL AND email_verified_at IS NULL) FROM users;"), "t")
        self.sql(project, f"UPDATE users SET profile_time_zone = 'Europe/Stockholm' WHERE id = '{identity_id(1)}';")
        # PHC-shaped synthetic fixture, not the hash of a real user's password.
        encoded = '$argon2id$v=19$m=19456,t=2,p=1$c3ludGhldGljLXNhbHQtMTY$' + 'A' * 43
        self.sql(project, f"INSERT INTO password_credentials (user_id, password_hash) VALUES ('{identity_id(1)}', '{encoded}');")
        self.reject_identity_write(project, f"INSERT INTO password_credentials (user_id, password_hash) VALUES ('{identity_id(99)}', '{encoded}');", "23503")
        self.reject_identity_write(project, f"INSERT INTO password_credentials (user_id, password_hash) VALUES ('{identity_id(1)}', '{encoded}');", "23505")
        self.reject_identity_write(project, f"INSERT INTO password_credentials (user_id, password_hash) VALUES ('{identity_id(2)}', 'plaintext');", "23514")
        self.reject_identity_write(project, f"DELETE FROM users WHERE id = '{identity_id(1)}';", "23001")
        self.reject_identity_write(project, f"UPDATE users SET profile_time_zone = '' WHERE id = '{identity_id(2)}';", "23514")
        self.reject_identity_write(project, f"UPDATE users SET created_at = 'infinity' WHERE id = '{identity_id(2)}';", "23514")
        self.assertEqual(self.sql(project, "SELECT datetime_precision FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'created_at';"), "6")

    def test_identity_session_ownership_and_rotation_storage(self):
        project = self.identity_project()
        self.seed_users(project)
        for session, owner, token, digest in [(11, 1, 101, 'aa'), (12, 2, 102, 'bb'), (13, 1, 103, 'cc')]:
            self.sql(project, f"""
                INSERT INTO user_sessions (id, user_id) VALUES ('{identity_id(session)}', '{identity_id(owner)}');
                INSERT INTO session_refresh_tokens (id, session_id, user_id, token_hash, created_at, expires_at)
                VALUES ('{identity_id(token)}', '{identity_id(session)}', '{identity_id(owner)}',
                    decode(repeat('{digest}', 32), 'hex'), '2026-10-24T12:00:00Z', '2026-11-24T12:00:00Z');
            """)
        base = f"INSERT INTO session_refresh_tokens (session_id, user_id, token_hash, created_at, expires_at) VALUES ('{identity_id(11)}', '{identity_id(1)}',"
        self.reject_identity_write(project, base + "decode(repeat('dd', 31), 'hex'), '2026-10-24T12:00:00Z', '2026-11-24T12:00:00Z');", "23514")
        self.reject_identity_write(project, base + "decode(repeat('aa', 32), 'hex'), '2026-10-24T12:00:00Z', '2026-11-24T12:00:00Z');", "23505")
        self.reject_identity_write(project, base + "decode(repeat('dd', 32), 'hex'), '2026-10-24T12:00:00Z', '2026-10-24T12:00:00Z');", "23514")
        self.reject_identity_write(project, f"""
            INSERT INTO session_refresh_tokens (session_id, user_id, token_hash, created_at, expires_at)
            VALUES ('{identity_id(12)}', '{identity_id(1)}', decode(repeat('dd', 32), 'hex'), '2026-10-24T12:00:00Z', '2026-11-24T12:00:00Z');
        """, "23503")
        for successor in [102, 103]:
            self.reject_identity_write(project, f"""
                UPDATE session_refresh_tokens SET used_at = '2026-10-24T13:00:00Z', replaced_by = '{identity_id(successor)}'
                WHERE id = '{identity_id(101)}';
            """, "23503")
        self.reject_identity_write(project, f"UPDATE session_refresh_tokens SET used_at = '2026-10-24T13:00:00Z' WHERE id = '{identity_id(101)}';", "23514")
        self.reject_identity_write(project, f"UPDATE session_refresh_tokens SET used_at = '2026-10-24T13:00:00Z', replaced_by = id WHERE id = '{identity_id(101)}';", "23514")
        self.sql(project, f"""
            INSERT INTO session_refresh_tokens (id, session_id, user_id, token_hash, created_at, expires_at)
            VALUES ('{identity_id(104)}', '{identity_id(11)}', '{identity_id(1)}', decode(repeat('dd', 32), 'hex'), '2026-10-24T13:00:00Z', '2026-11-24T12:00:00Z');
            UPDATE session_refresh_tokens SET used_at = '2026-10-24T13:00:00Z', replaced_by = '{identity_id(104)}' WHERE id = '{identity_id(101)}';
            UPDATE user_sessions SET revoked_at = '2026-10-24T14:00:00Z' WHERE id = '{identity_id(11)}';
        """)
        self.assertEqual(self.sql(project, "SELECT count(*) FROM session_refresh_tokens;"), "4")
        self.assertEqual(self.sql(project, f"SELECT used_at IS NOT NULL AND replaced_by = '{identity_id(104)}' FROM session_refresh_tokens WHERE id = '{identity_id(101)}';"), "t")
        self.assertEqual(self.sql(project, f"SELECT revoked_at IS NOT NULL FROM user_sessions WHERE id = '{identity_id(11)}';"), "t")

    def test_access_token_storage_constraints(self):
        project = self.identity_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('session_access_tokens') IS NOT NULL;"), "t", "Access-token table is missing")
        self.seed_users(project)
        self.sql(project, f"INSERT INTO user_sessions (id, user_id) VALUES ('{identity_id(11)}', '{identity_id(1)}'), ('{identity_id(12)}', '{identity_id(2)}');")
        self.sql(project, f"""
            INSERT INTO session_access_tokens (session_id, user_id, token_hash, expires_at)
            VALUES ('{identity_id(11)}', '{identity_id(1)}', decode(repeat('aa', 32), 'hex'), statement_timestamp() + INTERVAL '15 minutes');
        """)
        self.assertEqual(self.sql(project, """
            SELECT count(*) = 1 AND bool_and(
                id IS NOT NULL AND created_at IS NOT NULL AND expires_at > created_at
                AND octet_length(token_hash) = 32
            ) FROM session_access_tokens;
        """), "t")
        self.assertEqual(self.sql(project, "SELECT datetime_precision FROM information_schema.columns WHERE table_name = 'session_access_tokens' AND column_name = 'created_at';"), "6")

        base = f"INSERT INTO session_access_tokens (session_id, user_id, token_hash, created_at, expires_at) VALUES ('{identity_id(11)}', '{identity_id(1)}',"
        self.reject_access_write(project, base + "decode(repeat('bb', 31), 'hex'), statement_timestamp(), statement_timestamp() + INTERVAL '15 minutes');", "23514")
        self.reject_access_write(project, base + "decode(repeat('bb', 33), 'hex'), statement_timestamp(), statement_timestamp() + INTERVAL '15 minutes');", "23514")
        self.reject_access_write(project, base + "decode(repeat('aa', 32), 'hex'), statement_timestamp(), statement_timestamp() + INTERVAL '15 minutes');", "23505")
        self.reject_access_write(project, base + "decode(repeat('bb', 32), 'hex'), statement_timestamp(), statement_timestamp());", "23514")
        self.reject_access_write(project, base + "decode(repeat('bb', 32), 'hex'), '-infinity', '0001-01-01');", "23514")
        self.reject_access_write(project, base + "decode(repeat('bb', 32), 'hex'), '9999-01-01', 'infinity');", "23514")
        self.reject_access_write(project, base + "decode(repeat('bb', 32), 'hex'), '0001-01-01', '10000-01-01');", "23514")
        self.reject_access_write(project, f"""
            INSERT INTO session_access_tokens (session_id, user_id, token_hash, expires_at)
            VALUES ('{identity_id(99)}', '{identity_id(1)}', decode(repeat('bb', 32), 'hex'), statement_timestamp() + INTERVAL '15 minutes');
        """, "23503")
        self.reject_access_write(project, f"""
            INSERT INTO session_access_tokens (session_id, user_id, token_hash, expires_at)
            VALUES ('{identity_id(11)}', '{identity_id(2)}', decode(repeat('bb', 32), 'hex'), statement_timestamp() + INTERVAL '15 minutes');
        """, "23503")
        self.reject_access_write(project, f"DELETE FROM user_sessions WHERE id = '{identity_id(11)}';", "23001")
        for column in ["id", "session_id", "user_id", "token_hash", "created_at", "expires_at"]:
            self.reject_access_write(project, f"UPDATE session_access_tokens SET {column} = NULL;", "23502")
        self.reject_access_write(project, f"UPDATE session_access_tokens SET user_id = '{identity_id(2)}';", "23503")
        self.reject_access_write(project, "UPDATE session_access_tokens SET token_hash = decode(repeat('bb', 31), 'hex');", "23514")
        self.reject_access_write(project, f"UPDATE session_access_tokens SET session_id = '{identity_id(12)}';", "23503")
        self.reject_access_write(project, f"UPDATE user_sessions SET user_id = '{identity_id(2)}' WHERE id = '{identity_id(11)}';", "23503")
        mixed = f"""INSERT INTO session_access_tokens (session_id, user_id, token_hash, expires_at) VALUES
            ('{identity_id(11)}', '{identity_id(1)}', decode(repeat('bb', 32), 'hex'), statement_timestamp() + INTERVAL '15 minutes'),
            ('{identity_id(12)}', '{identity_id(1)}', decode(repeat('cc', 32), 'hex'), statement_timestamp() + INTERVAL '15 minutes');"""
        self.reject_access_write(project, mixed, "23503")
        self.reject_access_write(project, f"UPDATE users SET email_original = 'changed@example.test', email_address = 'changed@example.test' WHERE id = '{identity_id(1)}'; " + mixed, "23503")
        self.sql(project, f"""
            INSERT INTO session_access_tokens (session_id, user_id, token_hash, created_at, expires_at)
            VALUES ('{identity_id(11)}', '{identity_id(1)}', decode(repeat('bb', 32), 'hex'), '0001-01-01 00:00:00+00', '9999-12-31 23:59:59.999999+00');
        """)
        self.assertEqual(self.sql(project, "SELECT count(*) FROM session_access_tokens WHERE created_at = '0001-01-01 00:00:00+00' AND expires_at = '9999-12-31 23:59:59.999999+00';"), "1")



    def test_access_token_migration_preserves_existing_data(self):
        project = self.new_project()
        migrations = sorted(MIGRATIONS.glob("*.sql"))
        self.assertTrue(migrations, "Identity migrations are missing")
        legacy = [migration for migration in migrations if migration.name < "0009_session_access_tokens.sql"]
        for migration in legacy:
            result = self.identity_result(project, migration.read_text())
            self.assertEqual(result.returncode, 0, "Legacy migration application failed")
        self.seed_users(project)
        encoded = '$argon2id$v=19$m=19456,t=2,p=1$c3ludGhldGljLXNhbHQtMTY$' + 'A' * 43
        self.sql(project, f"""
            INSERT INTO password_credentials (user_id, password_hash) VALUES ('{identity_id(1)}', '{encoded}');
            INSERT INTO user_sessions (id, user_id, revoked_at) VALUES
                ('{identity_id(11)}', '{identity_id(1)}', NULL),
                ('{identity_id(12)}', '{identity_id(2)}', '2026-10-24T14:00:00Z');
            INSERT INTO session_refresh_tokens (id, session_id, user_id, token_hash, created_at, expires_at)
            VALUES ('{identity_id(101)}', '{identity_id(11)}', '{identity_id(1)}', decode(repeat('cc', 32), 'hex'),
                '2026-10-24T12:00:00Z', '2026-11-24T12:00:00Z');
        """)
        before = self.identity_snapshot(project)
        migration = MIGRATIONS / "0009_session_access_tokens.sql"
        self.assertTrue(migration.is_file(), "Access-token migration is missing")
        result = self.identity_result(project, migration.read_text())
        self.assertEqual(result.returncode, 0, "Access-token migration failed")
        self.assertEqual(self.identity_snapshot(project), before)
        self.assertEqual(self.sql(project, "SELECT to_regclass('session_access_tokens') IS NOT NULL;"), "t")

        after = self.access_snapshot(project)
        reapply = self.identity_result(project, migration.read_text())
        self.assertNotEqual(reapply.returncode, 0)
        self.assertIn("42P07", reapply.stderr)
        self.assertEqual(self.access_snapshot(project), after)

    def test_identity_migration_failure_rolls_back_partial_ddl(self):
        project = self.new_project()
        self.sql(project, "CREATE TABLE password_credentials (marker integer); INSERT INTO password_credentials VALUES (7);")
        migrations = sorted(MIGRATIONS.glob("*.sql"))
        self.assertTrue(migrations, "Versioned identity migration is missing")
        result = self.identity_result(project, migrations[0].read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("42P07", result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('users') IS NULL;"), "t")
        self.assertEqual(self.sql(project, "SELECT marker FROM password_credentials;"), "7")

    def accounts_project(self):
        project = self.identity_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('accounts') IS NOT NULL;"), "t", "Accounts migration is missing")
        self.seed_users(project)
        return project

    def accounts_snapshot(self, project):
        return self.identity_snapshot(project), self.sql(project, """
            SELECT md5(coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]')::text) FROM accounts a;
        """)

    def reject_account_write(self, project, query, sqlstate):
        before = self.accounts_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid account write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.accounts_snapshot(project), before)

    def test_accounts_supported_rows_and_archive_storage(self):
        project = self.accounts_project()
        for kind in ['normal', 'savings']:
            for currency in ['SEK', 'EUR', 'USD']:
                self.sql(project, f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(1)}', 'My account', '{kind}', '{currency}');")
        self.assertEqual(self.sql(project, "SELECT count(*) = 6 AND bool_and(archived_at IS NULL AND created_at IS NOT NULL AND updated_at IS NOT NULL) FROM accounts;"), "t")
        self.assertEqual(self.sql(project, "SELECT count(DISTINCT id) FROM accounts;"), "6")
        self.assertEqual(self.sql(project, "SELECT pg_typeof(id) FROM accounts LIMIT 1;"), "uuid")
        self.sql(project, "UPDATE accounts SET archived_at = '2026-10-24T12:00:00Z' WHERE account_type = 'savings';")
        self.assertEqual(self.sql(project, "SELECT count(*) FROM accounts WHERE archived_at IS NOT NULL;"), "3")
        self.assertEqual(self.sql(project, "SELECT count(*) FROM information_schema.columns WHERE table_name = 'accounts' AND column_name IN ('balance', 'opening_balance', 'opening_amount');"), "0")
        self.assertEqual(self.sql(project, "SELECT datetime_precision FROM information_schema.columns WHERE table_name = 'accounts' AND column_name = 'created_at';"), "6")
        self.reject_account_write(project, (MIGRATIONS / '0003_accounts.sql').read_text(), "42P07")

    def test_accounts_storage_guards_preserve_data(self):
        project = self.accounts_project()
        insert = f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(1)}',"
        for values in [
            "'', 'normal', 'SEK'", "'   ', 'normal', 'SEK'",
            "repeat('a', 256), 'normal', 'SEK'", "E'line\\nfeed', 'normal', 'SEK'",
            "'Name', 'credit', 'SEK'", "'Name', 'Normal', 'SEK'",
            "'Name', 'normal', 'GBP'", "'Name', 'normal', 'sek'",
            "'Name', 'normal', 'USD '",
        ]:
            self.reject_account_write(project, insert + values + ');', "23514")
        self.reject_account_write(project, insert + "NULL, 'normal', 'SEK');", "23502")
        for column in ['created_at', 'updated_at', 'archived_at']:
            self.reject_account_write(project, f"INSERT INTO accounts (user_id, name, account_type, currency, {column}) VALUES ('{identity_id(1)}', 'Name', 'normal', 'SEK', 'infinity');", "23514")

    def test_accounts_owner_and_composite_relationships(self):
        project = self.accounts_project()
        for account, owner in [(21, 1), (22, 2)]:
            self.sql(project, f"INSERT INTO accounts (id, user_id, name, account_type, currency) VALUES ('{identity_id(account)}', '{identity_id(owner)}', 'Owned account', 'normal', 'SEK');")
        self.reject_account_write(project, f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(99)}', 'Missing owner', 'normal', 'SEK');", "23503")
        self.reject_account_write(project, f"DELETE FROM users WHERE id = '{identity_id(1)}';", "23001")
        probe = """
            CREATE TABLE account_owner_probe (
                account_id UUID NOT NULL, user_id UUID NOT NULL,
                FOREIGN KEY (account_id, user_id) REFERENCES accounts (id, user_id)
            );
        """
        result = self.identity_result(project, probe + f"INSERT INTO account_owner_probe VALUES ('{identity_id(21)}', '{identity_id(1)}'); SELECT count(*) FROM account_owner_probe; DROP TABLE account_owner_probe;")
        self.assertEqual(result.returncode, 0)
        self.assertIn('1', result.stdout.splitlines())
        self.reject_account_write(project, probe + f"UPDATE accounts SET name = 'Attempted change' WHERE id = '{identity_id(21)}'; INSERT INTO account_owner_probe VALUES ('{identity_id(21)}', '{identity_id(2)}');", "23503")
        self.reject_account_write(project, probe + f"INSERT INTO account_owner_probe VALUES ('{identity_id(21)}', '{identity_id(1)}'); UPDATE accounts SET user_id = '{identity_id(2)}' WHERE id = '{identity_id(21)}';", "23503")
        self.assertEqual(self.sql(project, "SELECT to_regclass('account_owner_probe') IS NULL;"), "t")

    def test_accounts_migration_failure_rolls_back_partial_ddl(self):
        migration = MIGRATIONS / '0003_accounts.sql'
        self.assertTrue(migration.is_file(), "Accounts migration is missing")
        project = self.new_project()
        for prior in sorted(MIGRATIONS.glob('*.sql')):
            if prior.name < migration.name:
                result = self.identity_result(project, prior.read_text())
                self.assertEqual(result.returncode, 0)
        self.sql(project, "CREATE TABLE legacy_probe (marker integer); INSERT INTO legacy_probe VALUES (7); CREATE INDEX accounts_user_id_idx ON legacy_probe (marker);")
        before = self.identity_snapshot(project)
        result = self.identity_result(project, migration.read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("42P07", result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('accounts') IS NULL;"), "t")
        self.assertEqual(self.sql(project, "SELECT marker FROM legacy_probe;"), "7")
        self.assertEqual(self.identity_snapshot(project), before)

    def categories_project(self):
        project = self.accounts_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('categories') IS NOT NULL;"), "t", "Categories migration is missing")
        return project

    def categories_snapshot(self, project):
        return self.accounts_snapshot(project), self.sql(project, """
            SELECT md5(jsonb_agg(to_jsonb(c) ORDER BY id)::text) FROM categories c;
        """)

    def reject_category_write(self, project, query, sqlstate="55000"):
        before = self.categories_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid category write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.categories_snapshot(project), before)

    def test_categories_catalog_shared_and_reproducible(self):
        project = self.categories_project()
        catalog = [
            ("salary", "income", "Salary"),
            ("interest", "income", "Interest"),
            ("other_income", "income", "Other income"),
            ("housing", "expense", "Housing"),
            ("groceries", "expense", "Groceries"),
            ("transport", "expense", "Transport"),
            ("utilities", "expense", "Utilities"),
            ("health", "expense", "Health"),
            ("leisure", "expense", "Leisure"),
            ("other_expense", "expense", "Other expense"),
        ]
        expected = [f"10000000-0000-4000-8000-{i:012d}|{code}|{kind}|{name}"
                    for i, (code, kind, name) in enumerate(catalog, 1)]
        query = "SELECT id, code, kind, name FROM categories ORDER BY id;"
        self.assertEqual(self.sql(project, query).splitlines(), expected)
        self.assertEqual(self.sql(self.categories_project(), query).splitlines(), expected)
        self.assertEqual(self.sql(project, "SELECT count(*) FROM information_schema.columns WHERE table_name = 'categories' AND column_name = 'user_id';"), "0")
        self.assertEqual(self.sql(project, "SELECT count(*) FROM (SELECT u.id FROM users u CROSS JOIN categories c GROUP BY u.id HAVING count(*) = 10) visible;"), "2")

    def test_categories_catalog_rejects_mutations_atomically(self):
        project = self.categories_project()
        self.sql(project, f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(1)}', 'Original', 'normal', 'SEK');")
        # Explicit grants prove the guard, not merely missing SQL privileges.
        self.sql(project, """
            CREATE ROLE category_reader_a NOLOGIN;
            CREATE ROLE category_reader_b NOLOGIN;
            GRANT USAGE ON SCHEMA public TO category_reader_a, category_reader_b;
            GRANT SELECT ON categories TO category_reader_a, category_reader_b;
            GRANT INSERT, UPDATE, DELETE, TRUNCATE ON categories TO category_reader_a;
            GRANT SELECT, UPDATE ON accounts TO category_reader_a;
        """)
        query = "SELECT code, kind, name FROM categories ORDER BY code;"
        expected = self.sql(project, query).splitlines()
        for role in ['category_reader_a', 'category_reader_b']:
            result = self.identity_result(project, f"SET LOCAL ROLE {role}; " + query)
            self.assertEqual(result.returncode, 0)
            self.assertEqual([line for line in result.stdout.splitlines() if '|' in line], expected)
        for query in [
            "INSERT INTO categories (id, code, kind, name) VALUES (gen_random_uuid(), 'extra', 'expense', 'Extra');",
            "UPDATE categories SET name = 'Changed' WHERE code = 'salary';",
            "DELETE FROM categories WHERE code = 'interest';",
            "TRUNCATE categories;",
            "UPDATE categories SET kind = 'expense' WHERE false;",
        ]:
            # Native FK preflight rejects standalone TRUNCATE before the guard.
            code = '0A000' if query == 'TRUNCATE categories;' else '55000'
            self.reject_category_write(project, "SET LOCAL ROLE category_reader_a; UPDATE accounts SET name = 'Must roll back'; " + query, code)
        self.reject_category_write(project, (MIGRATIONS / '0004_categories.sql').read_text(), "42P07")

    def test_categories_kind_reference_target(self):
        project = self.categories_project()
        probe = """
            CREATE TABLE category_kind_probe (
                category_id UUID NOT NULL, kind TEXT NOT NULL,
                FOREIGN KEY (category_id, kind) REFERENCES categories (id, kind)
            );
        """
        valid = "INSERT INTO category_kind_probe SELECT id, kind FROM categories;"
        result = self.identity_result(project, probe + valid + "SELECT count(*) FROM category_kind_probe; DROP TABLE category_kind_probe;")
        self.assertEqual(result.returncode, 0)
        self.assertIn('10', result.stdout.splitlines())
        for query in [
            "INSERT INTO category_kind_probe SELECT id, 'expense' FROM categories WHERE code = 'salary';",
            "INSERT INTO category_kind_probe VALUES ('00000000-0000-0000-0000-000000000099', 'income');",
        ]:
            self.reject_category_write(project, probe + query, "23503")
        self.assertEqual(self.sql(project, "SELECT to_regclass('category_kind_probe') IS NULL;"), "t")

    def test_categories_migration_failure_rolls_back_seed_and_ddl(self):
        migration = MIGRATIONS / '0004_categories.sql'
        self.assertTrue(migration.is_file(), "Categories migration is missing")
        project = self.new_project()
        for prior in sorted(MIGRATIONS.glob('*.sql')):
            if prior.name < migration.name:
                self.assertEqual(self.identity_result(project, prior.read_text()).returncode, 0)
        self.seed_users(project)
        self.sql(project, f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(1)}', 'Preserved', 'normal', 'SEK');")
        self.sql(project, "CREATE FUNCTION reject_builtin_category_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$;")
        before = self.accounts_snapshot(project)
        definition = "SELECT pg_get_functiondef('reject_builtin_category_write()'::regprocedure);"
        original = self.sql(project, definition)
        result = self.identity_result(project, migration.read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("42723", result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('categories') IS NULL;"), "t")
        self.assertEqual(self.accounts_snapshot(project), before)
        self.assertEqual(self.sql(project, definition), original)

    def transactions_project(self):
        project = self.categories_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('transactions') IS NOT NULL;"), "t", "Transactions migration is missing")
        self.sql(project, f"""
            INSERT INTO accounts (id, user_id, name, account_type, currency) VALUES
                ('{identity_id(21)}', '{identity_id(1)}', 'Primary', 'normal', 'SEK'),
                ('{identity_id(22)}', '{identity_id(2)}', 'Other owner', 'normal', 'USD'),
                ('{identity_id(23)}', '{identity_id(1)}', 'Savings', 'savings', 'EUR'),
                ('{identity_id(24)}', '{identity_id(1)}', 'Secondary', 'normal', 'SEK');
        """)
        return project

    def transactions_snapshot(self, project):
        return self.categories_snapshot(project), self.sql(project, """
            SELECT md5(coalesce(jsonb_agg(to_jsonb(t) ORDER BY id), '[]')::text) FROM transactions t;
        """)

    def reject_transaction_write(self, project, query, sqlstate="23514"):
        before = self.transactions_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid transaction write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.transactions_snapshot(project), before)

    def test_transactions_supported_events_and_exact_storage(self):
        project = self.transactions_project()
        before = self.categories_snapshot(project)
        for kind, amount, currency, account, destination, received, received_currency, category in [
            ('income', '30000', 'SEK', 'NULL', 'NULL', 'NULL', 'NULL', "'10000000-0000-4000-8000-000000000001'"),
            ('expense', '20000', 'SEK', 'NULL', 'NULL', 'NULL', 'NULL', "'10000000-0000-4000-8000-000000000005'"),
            ('opening_balance', '-9223372036854775807', 'EUR', f"'{identity_id(23)}'", 'NULL', 'NULL', 'NULL', 'NULL'),
            ('interest_income', '105', 'EUR', f"'{identity_id(23)}'", 'NULL', 'NULL', 'NULL', "'10000000-0000-4000-8000-000000000002'"),
            ('transfer', '20000', 'SEK', f"'{identity_id(21)}'", f"'{identity_id(23)}'", '1795', "'EUR'", 'NULL'),
            ('transfer', '20000', 'SEK', f"'{identity_id(21)}'", f"'{identity_id(24)}'", '20000', "'SEK'", 'NULL'),
            ('income', '9223372036854775807', 'SEK', f"'{identity_id(21)}'", 'NULL', 'NULL', 'NULL', 'NULL'),
        ]:
            self.sql(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id, destination_account_id, received_amount_minor, received_currency, category_id) VALUES ('{identity_id(1)}', '{kind}', '2000-02-29', {amount}, '{currency}', {account}, {destination}, {received}, {received_currency}, {category});")
        self.assertEqual(self.sql(project, "SELECT count(DISTINCT id) = 7 AND bool_and(state = 'posted' AND created_at IS NOT NULL AND updated_at IS NOT NULL) FROM transactions;"), 't')
        self.assertEqual(self.sql(project, "SELECT pg_typeof(amount_minor), pg_typeof(occurred_on), pg_typeof(id) FROM transactions LIMIT 1;"), 'bigint|date|uuid')
        self.assertEqual(self.sql(project, "SELECT amount_minor FROM transactions WHERE kind = 'opening_balance';"), '-9223372036854775807')
        self.assertEqual(self.sql(project, "SELECT max(amount_minor) FROM transactions;"), '9223372036854775807')
        self.assertEqual(self.sql(project, "SELECT amount_minor, currency, received_amount_minor, received_currency FROM transactions WHERE kind = 'transfer' ORDER BY received_currency;"), '20000|SEK|1795|EUR\n20000|SEK|20000|SEK')
        self.assertEqual(self.sql(project, "SELECT count(*) FROM transactions WHERE account_id IS NULL;"), '2')
        self.assertEqual(self.categories_snapshot(project), before)

    def test_transactions_posted_and_voided_states(self):
        project = self.transactions_project()
        for state in ['posted', 'voided']:
            self.sql(project, f"INSERT INTO transactions (user_id, kind, state, occurred_on, amount_minor, currency, description) VALUES ('{identity_id(1)}', 'income', '{state}', '2000-01-01', 100, 'USD', 'Synthetic note');")
        self.assertEqual(self.sql(project, "SELECT state FROM transactions ORDER BY state;"), 'posted\nvoided')
        before = self.sql(project, "SELECT id, amount_minor, occurred_on FROM transactions ORDER BY id;")
        self.sql(project, "UPDATE transactions SET state = 'voided' WHERE state = 'posted';")
        self.assertEqual(self.sql(project, "SELECT count(*) FROM transactions WHERE state = 'voided';"), '2')
        self.assertEqual(self.sql(project, "SELECT id, amount_minor, occurred_on FROM transactions ORDER BY id;"), before)
        for state, code in [("'draft'", '23514'), ("'planned'", '23514'), ('NULL', '23502')]:
            self.reject_transaction_write(project, f"UPDATE transactions SET state = {state};", code)

    def test_transactions_row_guards_are_atomic(self):
        project = self.transactions_project()
        insert = f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency) VALUES ('{identity_id(1)}',"
        for values in [
            "'forecast', '2000-01-01', 100, 'SEK'", "'expense', '2000-01-01', 0, 'SEK'",
            "'expense', '2000-01-01', -1, 'SEK'",
            "'income', '2000-01-01', 100, 'GBP'", "'income', '2000-01-01', 100, 'sek'",
            "'income', 'infinity', 100, 'SEK'", "'income', '10000-01-01', 100, 'SEK'",
            "'opening_balance', '2000-01-01', 100, 'SEK'", "'interest_income', '2000-01-01', 100, 'SEK'",
        ]:
            self.reject_transaction_write(project, insert + values + ');')
        self.reject_transaction_write(project, insert + "'income', '2000-01-01', 9223372036854775808, 'SEK');", '22003')
        self.reject_transaction_write(project, insert + "'income', '2000-01-01', NULL, 'SEK');", '23502')
        self.reject_transaction_write(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id) VALUES ('{identity_id(1)}', 'opening_balance', '2000-01-01', -9223372036854775808, 'EUR', '{identity_id(23)}');")
        for column in ['created_at', 'updated_at']:
            self.reject_transaction_write(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, {column}) VALUES ('{identity_id(1)}', 'income', '2000-01-01', 100, 'SEK', 'infinity');")
        self.sql(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id, destination_account_id, received_amount_minor, received_currency) VALUES ('{identity_id(1)}', 'transfer', '2000-01-01', 100, 'SEK', '{identity_id(21)}', '{identity_id(24)}', 100, 'SEK');")
        for change in [
            'destination_account_id = NULL', 'account_id = destination_account_id',
            'received_amount_minor = NULL', 'received_currency = NULL',
            'received_amount_minor = 0', 'received_amount_minor = -1',
            "received_currency = 'GBP'", 'received_amount_minor = 99', "kind = 'expense'",
            "category_id = '10000000-0000-4000-8000-000000000001'",
        ]:
            self.reject_transaction_write(project, "UPDATE accounts SET name = 'Must roll back'; UPDATE transactions SET " + change + ';')

    def test_transactions_owner_and_category_relationships(self):
        project = self.transactions_project()
        self.sql(project, f"INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id, category_id) VALUES ('{identity_id(41)}', '{identity_id(1)}', 'income', '2000-01-01', 100, 'SEK', '{identity_id(21)}', '10000000-0000-4000-8000-000000000001');")
        for change in [f"user_id = '{identity_id(2)}'", f"user_id = '{identity_id(99)}'", f"account_id = '{identity_id(99)}'", "category_id = '10000000-0000-4000-8000-000000000005'"]:
            self.reject_transaction_write(project, 'UPDATE transactions SET ' + change + ';', '23503')
        self.reject_transaction_write(project, f"UPDATE accounts SET user_id = '{identity_id(2)}' WHERE id = '{identity_id(21)}';", '23503')
        self.reject_transaction_write(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id, destination_account_id, received_amount_minor, received_currency) VALUES ('{identity_id(1)}', 'transfer', '2000-01-01', 100, 'SEK', '{identity_id(21)}', '{identity_id(22)}', 100, 'USD');", '23503')
        self.reject_transaction_write(project, "UPDATE transactions SET category_kind = 'expense';", '428C9')
        self.sql(project, f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id, category_id) VALUES ('{identity_id(2)}', 'income', '2000-01-01', 100, 'USD', '{identity_id(22)}', '10000000-0000-4000-8000-000000000001');")
        self.reject_transaction_write(project, "UPDATE accounts SET name = 'Must roll back'; TRUNCATE categories;", '0A000')
        probe = "CREATE TABLE transaction_owner_probe (transaction_id UUID, user_id UUID, FOREIGN KEY (transaction_id, user_id) REFERENCES transactions (id, user_id));"
        result = self.identity_result(project, probe + f"INSERT INTO transaction_owner_probe VALUES ('{identity_id(41)}', '{identity_id(1)}'); SELECT count(*) FROM transaction_owner_probe; DROP TABLE transaction_owner_probe;")
        self.assertEqual(result.returncode, 0)
        self.assertIn('1', result.stdout.splitlines())
        self.reject_transaction_write(project, probe + f"INSERT INTO transaction_owner_probe VALUES ('{identity_id(41)}', '{identity_id(2)}');", '23503')
        self.assertEqual(self.sql(project, "SELECT to_regclass('transaction_owner_probe') IS NULL;"), 't')

    def test_transactions_same_currency_cross_owner_inserts_and_updates(self):
        project = self.transactions_project()
        self.sql(project, f"""
            INSERT INTO accounts (id, user_id, name, account_type, currency)
                VALUES ('{identity_id(25)}', '{identity_id(2)}', 'Other SEK', 'normal', 'SEK');
            INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id) VALUES
                ('{identity_id(45)}', '{identity_id(1)}', 'expense', '2000-01-01', 100, 'SEK', '{identity_id(21)}'),
                ('{identity_id(46)}', '{identity_id(2)}', 'expense', '2000-01-01', 100, 'SEK', '{identity_id(25)}');
            INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency,
                                      account_id, destination_account_id, received_amount_minor, received_currency)
                VALUES ('{identity_id(47)}', '{identity_id(1)}', 'transfer', '2000-01-01', 100, 'SEK',
                        '{identity_id(21)}', '{identity_id(24)}', 100, 'SEK');
            UPDATE transactions SET account_id = '{identity_id(24)}' WHERE id = '{identity_id(45)}';
        """)
        self.assertEqual(self.sql(project, f"SELECT account_id FROM transactions WHERE id = '{identity_id(45)}';"), identity_id(24))
        self.assertEqual(self.sql(project, "SELECT user_id, count(*) FROM transactions GROUP BY user_id ORDER BY user_id;"),
                         f"{identity_id(1)}|2\n{identity_id(2)}|1")
        invalid_writes = [
            f"INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency, account_id) VALUES ('{identity_id(1)}', 'expense', '2000-01-01', 100, 'SEK', '{identity_id(25)}');",
        ]
        for source, destination in [(25, 24), (21, 25)]:
            invalid_writes.append(f"""
                INSERT INTO transactions (user_id, kind, occurred_on, amount_minor, currency,
                                          account_id, destination_account_id, received_amount_minor, received_currency)
                    VALUES ('{identity_id(1)}', 'transfer', '2000-01-01', 100, 'SEK',
                            '{identity_id(source)}', '{identity_id(destination)}', 100, 'SEK');
            """)
        for event, change in [
            (45, f"account_id = '{identity_id(25)}'"),
            (45, f"user_id = '{identity_id(2)}'"),
            (47, f"account_id = '{identity_id(25)}'"),
            (47, f"destination_account_id = '{identity_id(25)}'"),
            (47, f"user_id = '{identity_id(2)}'"),
        ]:
            invalid_writes.append(f"UPDATE transactions SET {change} WHERE id = '{identity_id(event)}';")
        for query in invalid_writes:
            with self.subTest(query=query):
                self.reject_transaction_write(
                    project, f"UPDATE accounts SET name = 'Must roll back' WHERE id = '{identity_id(25)}';" + query,
                    '23503',
                )

    def test_transactions_reapply_and_failed_migration_preserve_data(self):
        migration = MIGRATIONS / '0005_transactions.sql'
        self.assertTrue(migration.is_file(), 'Transactions migration is missing')
        self.reject_transaction_write(self.transactions_project(), migration.read_text(), '42P07')
        project = self.new_project()
        for prior in sorted(MIGRATIONS.glob('*.sql')):
            if prior.name < migration.name:
                self.assertEqual(self.identity_result(project, prior.read_text()).returncode, 0)
        self.seed_users(project)
        self.sql(project, f"INSERT INTO accounts (user_id, name, account_type, currency) VALUES ('{identity_id(1)}', 'Preserved', 'normal', 'SEK'); CREATE TABLE legacy_probe (marker integer); INSERT INTO legacy_probe VALUES (7); CREATE INDEX transactions_user_id_idx ON legacy_probe (marker);")
        before = self.categories_snapshot(project)
        result = self.identity_result(project, migration.read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('42P07', result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('transactions') IS NULL;"), 't')
        self.assertEqual(self.sql(project, 'SELECT marker FROM legacy_probe;'), '7')
        self.assertEqual(self.categories_snapshot(project), before)

    def movements_project(self):
        project = self.transactions_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('account_movements') IS NOT NULL;"), 't', 'Movements migration is missing')
        for event, owner, account, currency in [(41, 1, 21, 'SEK'), (42, 2, 22, 'USD'), (43, 1, 23, 'EUR')]:
            self.sql(project, f"INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id) VALUES ('{identity_id(event)}', '{identity_id(owner)}', 'opening_balance', '2000-01-01', 100, '{currency}', '{identity_id(account)}');")
        return project

    def movement_insert(self, amount=100, event=41, owner=1, account=21, currency='SEK', date='2000-01-01'):
        return f"INSERT INTO account_movements (transaction_id, user_id, account_id, signed_amount_minor, currency, occurred_on) VALUES ('{identity_id(event)}', '{identity_id(owner)}', '{identity_id(account)}', {amount}, '{currency}', '{date}');"

    def movements_snapshot(self, project):
        return self.transactions_snapshot(project), self.sql(project, "SELECT md5(coalesce(jsonb_agg(to_jsonb(m) ORDER BY id), '[]')::text) FROM account_movements m;")

    def reject_movement_write(self, project, query, sqlstate):
        before = self.movements_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, 'Invalid movement write succeeded')
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.movements_snapshot(project), before)

    def test_movements_exact_postings_and_compensation_preserve_originals(self):
        project = self.movements_project()
        for amount in [100, -100, 0, -9223372036854775807, 9223372036854775807]:
            self.sql(project, self.movement_insert(amount=amount))
        original = self.movements_snapshot(project)
        original_ids = self.sql(project, "SELECT string_agg(quote_literal(id::text), ',') FROM account_movements;")
        self.sql(project, self.movement_insert(amount=-100))
        self.assertEqual(self.sql(project, "SELECT sum(signed_amount_minor) FROM account_movements;"), '-100')
        self.assertEqual(self.transactions_snapshot(project), original[0])
        self.assertEqual(self.sql(project, f"SELECT md5(jsonb_agg(to_jsonb(m) ORDER BY id)::text) FROM account_movements m WHERE id IN ({original_ids});"), original[1])
        self.assertEqual(self.sql(project, "SELECT count(DISTINCT id) = 6 AND bool_and(created_at IS NOT NULL) FROM account_movements;"), 't')
        self.assertEqual(self.sql(project, "SELECT pg_typeof(id), pg_typeof(signed_amount_minor), pg_typeof(occurred_on), pg_typeof(created_at) FROM account_movements LIMIT 1;"), 'uuid|bigint|date|timestamp with time zone')
        self.sql(project, self.movement_insert(event=42, owner=2, account=22, currency='USD', date='0001-01-01'))
        self.sql(project, self.movement_insert(event=43, account=23, currency='EUR', date='9999-12-31'))
        self.assertEqual(self.sql(project, "SELECT string_agg(DISTINCT currency, ',' ORDER BY currency) FROM account_movements;"), 'EUR,SEK,USD')

    def test_movements_support_transfer_legs_without_automatic_posting(self):
        project = self.movements_project()
        self.sql(project, f"INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency) VALUES ('{identity_id(44)}', '{identity_id(1)}', 'income', '2000-01-01', 100, 'SEK');")
        self.assertEqual(self.sql(project, 'SELECT count(*) FROM account_movements;'), '0')
        for event, destination, received, currency in [(45, 24, 20000, 'SEK'), (46, 23, 1795, 'EUR')]:
            self.sql(project, f"INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id, destination_account_id, received_amount_minor, received_currency) VALUES ('{identity_id(event)}', '{identity_id(1)}', 'transfer', '2000-01-01', 20000, 'SEK', '{identity_id(21)}', '{identity_id(destination)}', {received}, '{currency}');")
            result = self.identity_result(project, self.movement_insert(event=event, amount=-20000) + self.movement_insert(event=event, account=destination, amount=received, currency=currency))
            self.assertEqual(result.returncode, 0)
        self.assertEqual(self.sql(project, f"SELECT sum(signed_amount_minor) FROM account_movements WHERE transaction_id = '{identity_id(45)}';"), '0')
        self.assertEqual(self.sql(project, f"SELECT currency, signed_amount_minor FROM account_movements WHERE transaction_id = '{identity_id(46)}' ORDER BY currency;"), 'EUR|1795\nSEK|-20000')

    def test_movements_reject_invalid_storage_and_cross_owner_links(self):
        project = self.movements_project()
        self.sql(project, self.movement_insert())
        for changes, code in [
            ({'amount': -9223372036854775808}, '23514'),
            ({'amount': 9223372036854775808}, '22003'),
            ({'amount': 'NULL'}, '23502'),
            ({'currency': 'GBP'}, '23514'), ({'currency': 'sek'}, '23514'),
            ({'date': 'infinity'}, '23514'), ({'date': '-infinity'}, '23514'),
            ({'date': '0001-01-01 BC'}, '23514'), ({'date': '10000-01-01'}, '23514'),
            ({'account': 22}, '23503'), ({'event': 42}, '23503'),
            ({'owner': 2}, '23503'), ({'account': 99}, '23503'),
            ({'event': 99}, '23503'), ({'owner': 99}, '23503'),
        ]:
            self.reject_movement_write(project, self.movement_insert(**changes), code)
        columns = ['id', 'user_id', 'transaction_id', 'account_id', 'signed_amount_minor', 'currency', 'occurred_on', 'created_at']
        for column in columns:
            expressions = ['gen_random_uuid()' if name == 'id' else 'NULL' if name == column else name for name in columns]
            query = f"INSERT INTO account_movements ({', '.join(columns)}) SELECT {', '.join(expressions)} FROM account_movements;"
            if column == 'id':
                query = query.replace('gen_random_uuid()', 'NULL')
            self.reject_movement_write(project, query, '23502')
        for value in ['infinity', '-infinity', '0001-01-01 00:00:00+00 BC', '10000-01-01 00:00:00+00']:
            query = self.movement_insert().replace('occurred_on)', 'occurred_on, created_at)').replace("'2000-01-01');", f"'2000-01-01', '{value}');")
            self.reject_movement_write(project, query, '23514')
        for table, column, identifier in [('accounts', 'user_id', 21), ('transactions', 'user_id', 41)]:
            self.reject_movement_write(project, f"UPDATE {table} SET {column} = '{identity_id(2)}' WHERE id = '{identity_id(identifier)}';", '23503')
        for table, identifier in [('accounts', 21), ('transactions', 41), ('users', 1)]:
            self.reject_movement_write(project, f"DELETE FROM {table} WHERE id = '{identity_id(identifier)}';", '23001')

    def test_movements_same_currency_cross_owner_links_roll_back(self):
        project = self.movements_project()
        self.sql(project, f"""
            INSERT INTO accounts (id, user_id, name, account_type, currency)
                VALUES ('{identity_id(25)}', '{identity_id(2)}', 'Other SEK', 'normal', 'SEK');
            INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id)
                VALUES ('{identity_id(44)}', '{identity_id(2)}', 'opening_balance', '2000-01-01',
                        100, 'SEK', '{identity_id(25)}');
        """)
        self.sql(project, self.movement_insert() + self.movement_insert(owner=2, event=44, account=25))
        self.assertEqual(self.sql(project, "SELECT user_id, count(*), sum(signed_amount_minor) FROM account_movements GROUP BY user_id ORDER BY user_id;"),
                         f"{identity_id(1)}|1|100\n{identity_id(2)}|1|100")
        for owner, event, account in [(1, 41, 25), (1, 44, 21), (2, 44, 21), (2, 41, 25)]:
            with self.subTest(owner=owner, event=event, account=account):
                # Even a preceding valid posting must roll back with the invalid link.
                self.reject_movement_write(
                    project, self.movement_insert(amount=1)
                    + self.movement_insert(owner=owner, event=event, account=account), '23503',
                )

    def test_movements_append_only_guards_are_atomic(self):
        project = self.movements_project()
        self.sql(project, self.movement_insert())
        for mutation in [
            'UPDATE account_movements SET signed_amount_minor = 500',
            'DELETE FROM account_movements', 'TRUNCATE account_movements',
            'TRUNCATE transactions CASCADE', 'TRUNCATE accounts CASCADE',
            'TRUNCATE users CASCADE',
        ]:
            self.reject_movement_write(project, self.movement_insert(amount=-100) + mutation + ';', '55000')
        self.reject_movement_write(project, self.movement_insert() + 'UPDATE account_movements SET signed_amount_minor = signed_amount_minor WHERE false;', '55000')

    def test_movements_indexes_reapply_and_failed_ddl_preserve_data(self):
        project = self.movements_project()
        self.sql(project, self.movement_insert())
        indexes = self.sql(project, "SELECT indexdef FROM pg_indexes WHERE tablename = 'account_movements' ORDER BY indexname;")
        self.assertIn('(user_id, account_id, occurred_on)', indexes)
        self.assertIn('(transaction_id, user_id)', indexes)
        self.assertIn('(user_id, occurred_on)', indexes)
        migration = MIGRATIONS / '0006_account_movements.sql'
        self.reject_movement_write(project, migration.read_text(), '42P07')
        project = self.new_project()
        for prior in sorted(MIGRATIONS.glob('*.sql')):
            if prior.name < migration.name:
                self.assertEqual(self.identity_result(project, prior.read_text()).returncode, 0)
        self.seed_users(project)
        self.sql(project, "CREATE TABLE legacy_probe (marker integer); INSERT INTO legacy_probe VALUES (7); CREATE INDEX account_movements_transaction_idx ON legacy_probe (marker);")
        before = self.transactions_snapshot(project)
        result = self.identity_result(project, migration.read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('42P07', result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('account_movements') IS NULL AND to_regprocedure('reject_account_movement_mutation()') IS NULL;"), 't')
        self.assertEqual(self.sql(project, 'SELECT marker FROM legacy_probe;'), '7')
        self.assertEqual(self.transactions_snapshot(project), before)

    def audit_project(self):
        project = self.movements_project()
        self.assertEqual(self.sql(project, "SELECT to_regclass('transaction_audit') IS NOT NULL;"), 't', 'Audit migration is missing')
        self.sql(project, self.movement_insert())
        return project

    def audit_insert(self, owner=1, actor=1, event=41, action='edit', fields="ARRAY['description']::TEXT[]"):
        return f"INSERT INTO transaction_audit (user_id, actor_id, transaction_id, action, changed_fields) VALUES ('{identity_id(owner)}', '{identity_id(actor)}', '{identity_id(event)}', '{action}', {fields});"

    def audit_snapshot(self, project):
        return self.movements_snapshot(project), self.sql(project, "SELECT md5(coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]')::text) FROM transaction_audit a;")

    def reject_audit_write(self, project, query, sqlstate):
        before = self.audit_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, 'Invalid audit write succeeded')
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.audit_snapshot(project), before)

    def test_audit_minimal_events_and_timestamp_defaults(self):
        project = self.audit_project()
        before = self.movements_snapshot(project)
        for action in ['create', 'edit', 'void']:
            self.sql(project, self.audit_insert(action=action, fields="ARRAY[]::TEXT[]"))
        self.sql(project, self.audit_insert(owner=2, actor=2, event=42))
        fields = "ARRAY['kind', 'state', 'occurred_on', 'amount_minor', 'currency', 'account_id', 'destination_account_id', 'received_amount_minor', 'received_currency', 'category_id', 'description']"
        self.sql(project, self.audit_insert(fields=fields))
        self.assertEqual(self.movements_snapshot(project), before)
        self.assertEqual(self.sql(project, "SELECT count(*) = 5 AND count(DISTINCT id) = 5 AND bool_and(created_at IS NOT NULL) FROM transaction_audit;"), 't')
        self.assertEqual(self.sql(project, "SELECT string_agg(column_name, ',' ORDER BY ordinal_position) FROM information_schema.columns WHERE table_name = 'transaction_audit';"), 'id,user_id,actor_id,transaction_id,action,changed_fields,created_at')
        self.assertEqual(self.sql(project, "SELECT data_type, datetime_precision FROM information_schema.columns WHERE table_name = 'transaction_audit' AND column_name = 'created_at';"), 'timestamp with time zone|6')
        query = self.audit_insert().replace('changed_fields)', 'changed_fields, created_at)').replace("ARRAY['description']::TEXT[]);", "ARRAY['description']::TEXT[], '2000-01-01 02:00:00.123456+02');")
        self.sql(project, query + query)
        self.assertEqual(self.sql(project, "SELECT count(*) FROM transaction_audit WHERE created_at = '2000-01-01 00:00:00.123456+00';"), '2')
        self.assertEqual(self.sql(project, "SELECT count(*) FROM transaction_audit WHERE created_at BETWEEN statement_timestamp() - INTERVAL '1 minute' AND statement_timestamp();"), '5')
        # A financial write alone does not silently promise automatic auditing.
        before = self.audit_snapshot(project)[1]
        self.sql(project, "UPDATE transactions SET description = 'Synthetic change';")
        self.assertEqual(self.audit_snapshot(project)[1], before)

    def test_audit_rejects_cross_owner_actor_and_sensitive_summaries(self):
        project = self.audit_project()
        self.sql(project, self.audit_insert())
        for changes, code in [
            ({'owner': 2, 'actor': 2}, '23503'),
            ({'actor': 2}, '23514'), ({'actor': 99}, '23514'),
            ({'event': 42}, '23503'), ({'event': 99}, '23503'),
            ({'owner': 99, 'actor': 99}, '23503'),
            ({'action': 'delete'}, '23514'), ({'action': 'EDIT'}, '23514'),
            ({'fields': "ARRAY['private description']"}, '23514'),
            ({'fields': "ARRAY['amount_minor=500']"}, '23514'),
            ({'fields': "ARRAY['user_id']"}, '23514'),
            ({'fields': "ARRAY['Description']"}, '23514'),
            ({'fields': "ARRAY[NULL]::TEXT[]"}, '23514'),
            ({'fields': "ARRAY[['description']]"}, '23514'),
            ({'fields': "array_fill('description'::TEXT, ARRAY[12])"}, '23514'),
        ]:
            self.reject_audit_write(project, self.audit_insert(**changes), code)
        columns = ['id', 'user_id', 'actor_id', 'transaction_id', 'action', 'changed_fields', 'created_at']
        for column in columns:
            expressions = ['NULL' if name == column else 'gen_random_uuid()' if name == 'id' else name for name in columns]
            self.reject_audit_write(project, f"INSERT INTO transaction_audit ({', '.join(columns)}) SELECT {', '.join(expressions)} FROM transaction_audit;", '23502')
        for value in ['infinity', '-infinity', '0001-01-01 00:00:00+00 BC', '10000-01-01 00:00:00+00']:
            query = self.audit_insert().replace('changed_fields)', 'changed_fields, created_at)').replace("ARRAY['description']::TEXT[]);", f"ARRAY['description']::TEXT[], '{value}');")
            self.reject_audit_write(project, query, '23514')
        for value in ['0001-01-01 00:00:00+00', '9999-12-31 23:59:59.999999+00']:
            query = self.audit_insert().replace('changed_fields)', 'changed_fields, created_at)').replace("ARRAY['description']::TEXT[]);", f"ARRAY['description']::TEXT[], '{value}');")
            self.sql(project, query)
        self.reject_audit_write(project, f"UPDATE transactions SET user_id = '{identity_id(2)}' WHERE id = '{identity_id(41)}';", '23503')
        # Event 43 has audit but no movements: the audit FK itself blocks deletion.
        self.sql(project, self.audit_insert(event=43))
        self.reject_audit_write(project, f"DELETE FROM transactions WHERE id = '{identity_id(43)}';", '23001')

    def test_audit_append_only_rejections_preserve_all_data(self):
        project = self.audit_project()
        self.sql(project, self.audit_insert())
        for mutation in [
            "UPDATE transaction_audit SET action = 'void'",
            'UPDATE transaction_audit SET action = action WHERE false',
            'DELETE FROM transaction_audit', 'TRUNCATE transaction_audit',
            'TRUNCATE transactions CASCADE', 'TRUNCATE users CASCADE',
        ]:
            self.reject_audit_write(project, self.audit_insert() + mutation + ';', '55000')

    def test_audit_indexes_reapply_and_failed_ddl_preserve_data(self):
        project = self.audit_project()
        self.sql(project, self.audit_insert())
        indexes = self.sql(project, "SELECT indexdef FROM pg_indexes WHERE tablename = 'transaction_audit' ORDER BY indexname;")
        self.assertIn('(user_id, transaction_id, created_at)', indexes)
        migration = MIGRATIONS / '0007_transaction_audit.sql'
        self.reject_audit_write(project, migration.read_text(), '42P07')
        project = self.new_project()
        for prior in sorted(MIGRATIONS.glob('*.sql')):
            if prior.name < migration.name:
                self.assertEqual(self.identity_result(project, prior.read_text()).returncode, 0)
        self.seed_users(project)
        self.sql(project, "CREATE TABLE legacy_probe (marker integer); INSERT INTO legacy_probe VALUES (7); CREATE INDEX transaction_audit_transaction_date_idx ON legacy_probe (marker);")
        before = self.movements_snapshot(project)
        result = self.identity_result(project, migration.read_text())
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('42P07', result.stderr)
        self.assertEqual(self.sql(project, "SELECT to_regclass('transaction_audit') IS NULL AND to_regprocedure('reject_transaction_audit_mutation()') IS NULL;"), 't')
        self.assertEqual(self.sql(project, 'SELECT marker FROM legacy_probe;'), '7')
        self.assertEqual(self.movements_snapshot(project), before)

    def financial_insert(self, event=90, kind='income', account=21, currency='SEK', destination=None, received_currency=None):
        account_sql = 'NULL' if account is None else f"'{identity_id(account)}'"
        destination_sql = 'NULL' if destination is None else f"'{identity_id(destination)}'"
        received = 'NULL' if destination is None else '100'
        received_currency_sql = 'NULL' if received_currency is None else f"'{received_currency}'"
        return f"INSERT INTO transactions (id, user_id, kind, occurred_on, amount_minor, currency, account_id, destination_account_id, received_amount_minor, received_currency) VALUES ('{identity_id(event)}', '{identity_id(1)}', '{kind}', '2000-01-01', 100, '{currency}', {account_sql}, {destination_sql}, {received}, {received_currency_sql});"

    def test_financial_currency_and_savings_rules_on_insert_and_edit(self):
        project = self.movements_project()
        for event, changes in enumerate([
            {}, {'account': None}, {'kind': 'expense', 'account': 23, 'currency': 'EUR'},
            {'kind': 'interest_income', 'account': 23, 'currency': 'EUR'},
            {'kind': 'interest_income'}, {'kind': 'opening_balance', 'account': 23, 'currency': 'EUR'},
            {'kind': 'transfer', 'destination': 23, 'received_currency': 'EUR'},
        ], 61):
            self.sql(project, self.financial_insert(event=event, **changes))
        for index, changes in enumerate([
            {'currency': 'USD'}, {'account': 23, 'currency': 'EUR'},
            {'kind': 'expense', 'account': 23},
            {'kind': 'transfer', 'currency': 'USD', 'destination': 23, 'received_currency': 'EUR'},
            {'kind': 'transfer', 'destination': 23, 'received_currency': 'USD'},
        ]):
            with self.subTest(changes=changes):
                self.reject_audit_write(project, self.financial_insert(event=90 + index, **changes), '23503')
        for query in [
            f"UPDATE transactions SET currency = 'EUR' WHERE id = '{identity_id(61)}';",
            f"UPDATE transactions SET account_id = '{identity_id(23)}', currency = 'EUR' WHERE id = '{identity_id(62)}';",
            f"UPDATE transactions SET kind = 'income' WHERE id = '{identity_id(63)}';",
            f"UPDATE transactions SET received_currency = 'USD' WHERE id = '{identity_id(67)}';",
            self.movement_insert(currency='USD'),
        ]:
            with self.subTest(query=query):
                self.reject_audit_write(project, query, '23503')
        # Assignment to a compatible normal account remains valid.
        self.sql(project, f"UPDATE transactions SET account_id = '{identity_id(21)}' WHERE id = '{identity_id(62)}';")
        self.sql(project, self.movement_insert(amount=-100) + self.movement_insert(amount=0))
        self.assertEqual(self.sql(project, 'SELECT count(*) FROM account_movements;'), '2')
        self.assertEqual(self.sql(project, 'SELECT count(*) FROM transaction_audit;'), '0')

    def test_financial_account_changes_preserve_existing_history(self):
        project = self.movements_project()
        self.sql(project, self.financial_insert(event=61) + self.movement_insert())
        for state in ['posted', 'voided']:
            self.sql(project, f"UPDATE transactions SET state = '{state}' WHERE id = '{identity_id(61)}';")
            self.reject_audit_write(project, f"UPDATE accounts SET account_type = 'savings' WHERE id = '{identity_id(21)}';", '23001')
        self.reject_audit_write(project, f"UPDATE accounts SET currency = 'USD' WHERE id = '{identity_id(21)}';", '23001')
        # A movement pins its account currency even if its current event is accountless.
        self.sql(project, f"UPDATE transactions SET kind = 'expense', account_id = NULL WHERE id = '{identity_id(41)}'; UPDATE transactions SET account_id = NULL WHERE id = '{identity_id(61)}';")
        self.reject_audit_write(project, f"UPDATE accounts SET currency = 'USD' WHERE id = '{identity_id(21)}';", '23001')
        self.sql(project, f"UPDATE accounts SET account_type = 'savings' WHERE id = '{identity_id(21)}'; UPDATE accounts SET account_type = 'normal' WHERE id = '{identity_id(23)}'; UPDATE accounts SET currency = 'USD', account_type = 'savings' WHERE id = '{identity_id(24)}';")
        self.assertEqual(self.sql(project, f"SELECT currency, account_type FROM accounts WHERE id = '{identity_id(24)}';"), 'USD|savings')

    def financial_legacy_project(self):
        project = self.new_project()
        for migration in sorted(MIGRATIONS.glob('*.sql')):
            if migration.name < '0008_financial_constraints.sql':
                self.assertEqual(self.identity_result(project, migration.read_text()).returncode, 0)
        self.seed_users(project)
        self.sql(project, f"INSERT INTO accounts (id, user_id, name, account_type, currency) VALUES ('{identity_id(21)}', '{identity_id(1)}', 'Primary', 'normal', 'SEK'), ('{identity_id(23)}', '{identity_id(1)}', 'Savings', 'savings', 'EUR');")
        return project

    def test_financial_migration_validates_existing_data_atomically(self):
        migration = MIGRATIONS / '0008_financial_constraints.sql'
        self.assertTrue(migration.is_file(), 'Financial constraints migration is missing')
        for invalid in [None, self.financial_insert(currency='USD'), self.financial_insert(account=23, currency='EUR'), self.financial_insert(kind='transfer', destination=23, received_currency='USD'), self.movement_insert(currency='USD')]:
            project = self.financial_legacy_project()
            self.sql(project, self.financial_insert(event=41, kind='opening_balance'))
            if invalid:
                self.sql(project, invalid)
            before = self.audit_snapshot(project)
            result = self.identity_result(project, migration.read_text())
            if invalid:
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('23503', result.stderr)
                self.assertEqual(self.audit_snapshot(project), before)
                self.assertEqual(self.sql(project, "SELECT count(*) FROM information_schema.columns WHERE table_name = 'transactions' AND column_name = 'income_account_type';"), '0')
                self.assertEqual(self.sql(project, "SELECT count(*) FROM pg_constraint WHERE conname IN ('accounts_owner_currency', 'accounts_owner_type');"), '0')
            else:
                self.assertEqual(result.returncode, 0)
                self.assertEqual(self.audit_snapshot(project)[0][0][0], before[0][0][0])
                self.assertEqual(self.sql(project, 'SELECT count(*), sum(amount_minor) FROM transactions;'), '1|100')
                self.assertEqual(self.sql(project, "SELECT count(*) FROM pg_constraint WHERE conrelid IN ('transactions'::regclass, 'account_movements'::regclass) AND NOT convalidated;"), '0')
                indexes = self.sql(project, "SELECT indexdef FROM pg_indexes WHERE tablename = 'transactions';")
                self.assertIn('(user_id, occurred_on)', indexes)
                self.reject_audit_write(project, migration.read_text(), '42P07')

    def financial_wait_for(self, project, predicate):
        import time
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if self.sql(project, 'SELECT ' + predicate + ';') == 't':
                return
            time.sleep(0.05)
        self.fail('Concurrent SQL session did not reach the expected lock state')

    def test_financial_concurrent_account_changes_cannot_bypass_guards(self):
        from concurrent.futures import ThreadPoolExecutor
        project = self.transactions_project()
        account = identity_id(24)
        for change in ["account_type = 'savings'", "currency = 'USD'"]:
            for account_first in [True, False]:
                self.sql(project, f"UPDATE accounts SET account_type = 'normal', currency = 'SEK' WHERE id = '{account}';")
                update = f"UPDATE accounts SET {change} WHERE id = '{account}';"
                insert = self.financial_insert(account=24)
                holder, waiter = (update, insert) if account_first else (insert, update)
                with ThreadPoolExecutor(max_workers=2) as pool:
                    first = pool.submit(self.identity_result, project, "SET LOCAL application_name = 'financial_holder'; " + holder + ' SELECT pg_sleep(5);')
                    self.financial_wait_for(project, "EXISTS (SELECT 1 FROM pg_stat_activity WHERE application_name = 'financial_holder' AND wait_event = 'PgSleep')")
                    second = pool.submit(self.identity_result, project, "SET LOCAL application_name = 'financial_waiter'; SET LOCAL lock_timeout = '12s'; " + waiter)
                    self.financial_wait_for(project, "EXISTS (SELECT 1 FROM pg_stat_activity WHERE application_name = 'financial_waiter' AND cardinality(pg_blocking_pids(pid)) > 0)")
                    self.assertEqual(first.result(timeout=30).returncode, 0)
                    rejected = second.result(timeout=30)
                    self.assertNotEqual(rejected.returncode, 0)
                    self.assertIn('23503' if account_first else '23001', rejected.stderr)
                if not account_first:
                    self.sql(project, f"UPDATE transactions SET account_id = NULL WHERE id = '{identity_id(90)}'; DELETE FROM transactions WHERE id = '{identity_id(90)}';")
                self.assertEqual(self.sql(project, 'SELECT count(*) FROM transaction_audit;'), '0')

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
