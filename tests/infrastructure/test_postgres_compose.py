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

    def reject_identity_write(self, project, query, sqlstate):
        before = self.identity_snapshot(project)
        result = self.identity_result(project, query)
        self.assertNotEqual(result.returncode, 0, "Invalid identity write succeeded")
        self.assertIn(sqlstate, result.stderr)
        self.assertEqual(self.identity_snapshot(project), before)

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
            self.reject_category_write(project, "SET LOCAL ROLE category_reader_a; UPDATE accounts SET name = 'Must roll back'; " + query)
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
