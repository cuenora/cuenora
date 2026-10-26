# Development environment

The Rust backend workspace contains two crates:
`cuenora-domain` at `backend/crates/domain` and `cuenora-api` at
`backend/crates/api`. The domain library has no dependencies and implements
`Money`, `Currency` and typed monetary errors; see the [money specification](../product/money.md).
The API uses Axum and Tokio for a minimal local HTTP server. Financial application
behavior and database access are not implemented yet.

## Prerequisites

- Git
- Python 3 (for lightweight repository checks)
- Docker and a Dev Containers-compatible editor, **or** a local Rust toolchain

The Dev Container is intended for **Rust backend development and repository tooling**.
It does not install Android SDK or Xcode. Android and iOS development environments
will be configured when the mobile project is initialized.

## Option A: Dev Container (recommended for backend development)

1. Clone the repository:

   ```bash
   git clone https://github.com/cuenora/cuenora.git
   cd cuenora
   ```

2. Install Docker and a Dev Containers-compatible editor.
3. Open this repository in the editor.
4. Choose **Dev Containers: Reopen in Container**.
5. Verify the toolchain and repository checks in the container:

   ```bash
   cd backend
   rustup show
   rustc --version
   cargo --version
   cargo clippy --version
   cargo fmt --version
   cd ..
   python3 scripts/check_repository.py
   ```

The development container image is based on Debian Bookworm and contains Rust
utilities. The Rust toolchain is pinned to `1.99.1` in
`backend/rust-toolchain.toml`, including Rustfmt and Clippy. Run Rust commands
from `backend/` so rustup selects this toolchain. Ensure local version managers
or `RUSTUP_TOOLCHAIN` overrides do not select a different compiler.

## Option B: Local installation

Install Rust through [rustup](https://rustup.rs/). From the repository root:

```bash
cd backend
rustup show
cargo clippy --version
cargo fmt --version
cd ..
python3 scripts/check_repository.py
```

From `backend/`, validate and build the workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --locked
```

The API has an HTTP-level liveness route test. The domain has 20 public-interface
test groups for exact parsing, arithmetic, bounds and half-even quantization.
Application financial use cases, database behavior and their tests remain pending.
Run just the monetary acceptance tests from `backend/` with
`cargo test -p cuenora-domain --test money --locked`. If an ambient override selects
another compiler, prefix Cargo commands with `rustup run 1.99.1` to use the pin.

## Run the local API

From `backend/`, start the server:

```bash
cargo run -p cuenora-api --locked
```

It listens only on `127.0.0.1:3000` and prints its local address after binding.
From another terminal, check the process:

```bash
curl --fail --include http://127.0.0.1:3000/health/live
```

Expect status `200`, content type `text/plain; charset=utf-8`, and body `OK`.
The response has no trailing newline. PostgreSQL, `.env`, and credentials are not
required; liveness does not check database readiness. No readiness route exists.
If port 3000 is occupied, startup exits with an error; stop the conflicting local
process before retrying. Stop the API with Ctrl+C.

The endpoint is described in [`contracts/openapi.yaml`](../../contracts/openapi.yaml).
The local-only address is fixed for this first endpoint; deployment and runtime
configuration are separate tasks.

## Rust formatting and lint conventions

Use the Rustfmt and Clippy versions shipped with the pinned backend toolchain.
Run these commands from `backend/`.

To apply formatting locally:

```bash
cargo fmt --all
```

Use `cargo fmt --all -- --check` to validate without modifying files, as CI does.
Use Rustfmt defaults; no custom `rustfmt.toml` is needed for the current workspace.

Run Clippy for the entire workspace with the same scope and warning policy as CI:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Fix warnings before submitting a PR. Avoid blanket lint suppressions and introduce
custom lint groups only when a concrete project requirement justifies them.

A task runner (`make` or `just`) is deferred: the current workflow is a short list
of direct commands and does not need additional orchestration. Keeping Cargo
commands explicit avoids another prerequisite and duplicate command definitions.
Revisit this decision when repeated multi-step tasks justify a shared entry point.

## Local PostgreSQL

Install Docker Engine or Docker Desktop with Docker Compose v2 or later. Run the
following from the repository root against your local Docker daemon; the Dev
Container does not configure Docker access automatically.

The development-only service uses the official `postgres:18-bookworm` image,
a project-scoped named volume, and a host port bound only to `127.0.0.1`.
There is no backend database connection or application schema yet. Use synthetic
data only. The `cuenora` bootstrap role is a local database administrator, not a
production application role.

In Bash, generate a password for a new database without printing it:

```bash
export CUENORA_POSTGRES_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
docker compose -f infra/compose.yaml config --quiet
docker compose -f infra/compose.yaml up -d --wait --wait-timeout 90
docker compose -f infra/compose.yaml ps
```

Keep the password in your current shell, or store it in a local secret manager
and restore it for later sessions. Do not regenerate it when reusing an existing
volume: image initialization variables only apply to an empty data directory.
Never commit passwords or print rendered Compose configuration; use
`config --quiet` for validation.

### Optional local environment file

[`.env.example`](../../.env.example) lists the variables currently supported by
Compose. Its password is intentionally empty: the untouched example cannot start
a database unless a password is supplied from the shell. No backend application
configuration is implemented yet.

From the repository root, copy the template only if `.env` does not already exist:

```bash
(umask 077; set -o noclobber; cat .env.example > .env)
```

This Bash command refuses to overwrite an existing file and creates a private
copy readable only by your user. Edit `.env` locally and fill
`CUENORA_POSTGRES_PASSWORD`; keep or change `CUENORA_POSTGRES_PORT` as needed.
For a new database, use a generated password from a password manager. For an
existing database volume, use its original password rather than generating a new
one. Keep the file private; `.env` is ignored by Git, while `.env.example` is
tracked without credentials. Never paste passwords into documentation or PRs.

Load the file explicitly with every Compose command in this mode:

```bash
docker compose --env-file .env -f infra/compose.yaml config --quiet
docker compose --env-file .env -f infra/compose.yaml up -d --wait --wait-timeout 90
docker compose --env-file .env -f infra/compose.yaml ps
docker compose --env-file .env -f infra/compose.yaml exec postgres \
  psql -h 127.0.0.1 -U cuenora -d cuenora -W
docker compose --env-file .env -f infra/compose.yaml down
```

Exported shell variables take precedence over `.env` entries. If you previously
used the shell-based setup and now want the file's values, first run:

```bash
unset CUENORA_POSTGRES_PASSWORD CUENORA_POSTGRES_PORT
```

Use `config --quiet`; plain `config` renders the password. The original
shell-export workflow above still works without an env file.

### Connection and lifecycle

The database and user are both `cuenora`, and the default connection is
`127.0.0.1:5432`. To use a different port, set `CUENORA_POSTGRES_PORT` before
starting the service. Connect over TCP using the container's bundled client and
enter the password at its prompt:

```bash
docker compose -f infra/compose.yaml exec postgres \
  psql -h 127.0.0.1 -U cuenora -d cuenora -W
```

Stop containers and the network while preserving the database volume:

```bash
docker compose -f infra/compose.yaml down
```

Only when intentionally discarding this project's local database, remove its
volume too:

```bash
docker compose -f infra/compose.yaml down --volumes
```

Compose defaults to project `cuenora-local`. For separate checkouts or disposable
environments, use a unique `-p <project-name>` with every Compose command and a
different `CUENORA_POSTGRES_PORT`. Never point integration tests at the default
development database. The isolated smoke test chooses fresh projects and free
ports, checks password rejection and persistence, and removes its own volumes:

```bash
python3 tests/infrastructure/test_postgres_compose.py
```

### Registration persistence adapter (P04.02, HTTP pending)

Run the SQLx adapter checks only against a fresh disposable database:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_registration_store_against_disposable_postgres -v
```

The existing Compose harness applies migrations and passes random test credentials
through environment variables, never arguments. It explicitly executes four Rust
database tests ignored by normal Cargo tests; CI includes this case in the same
infrastructure suite. No developer `.env` or application database is used. Do not
manually point the ignored tests at a personal database. See [ADR-010](../architecture/decisions/010-atomic-registration-storage.md).

These are persistence-adapter tests, not registration HTTP tests. The running API
still exposes only liveness; application DB configuration remains pending.

### Fresh migration verification (P03.11)

For the dedicated fresh-instance check:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_all_migrations_reproduce_schema_on_fresh_instances -v
```

This applies every SQL migration in filename order to two independent, disposable
PostgreSQL instances. It checks all nine application tables, validated constraints,
the final financial constraints, identical schema-only dumps and category seeds,
and empty non-catalog tables. Only the dump's random psql safety-token lines are
excluded from comparison. Existing cleanup checks verify removal of test
containers, volumes and networks. CI runs this test as part of the existing suite;
no new migration runner or application database connection is introduced.

**Verified locally on 2026-10-25:** the focused test passed (1 test, 19.848 s)
and the complete infrastructure suite passed (34 tests, 698.182 s), using the
local `postgres:18-bookworm` image. A disposable mutation probe that omitted
`0008_financial_constraints.sql` failed at the expected final-constraint assertion,
confirming that the new test detects an incomplete chain. Cleanup completed in
all three runs. This is a test-only addition over existing schema behavior, so
there is no production behavior change requiring a RED-to-GREEN implementation.

### SQL constraints and owned relationships (P03.12)

Run the focused same-currency ownership checks with:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_transactions_same_currency_cross_owner_inserts_and_updates \
  PostgresComposeTest.test_movements_same_currency_cross_owner_links_roll_back -v
```

Both users have valid SEK accounts and events. Positive controls prove that their
own relationships work; negative cases reject foreign account/event references
without relying on a currency mismatch. Transaction cases cover insertion, edits,
owner changes and both transfer sides. Movement cases isolate account ownership
from transaction ownership in both directions. Rejected requests must return a
nonzero exit status and SQLSTATE `23503`; existing snapshot helpers also verify
that preceding valid writes roll back and stored data is unchanged.

The full suite also retains the earlier constraint checks:

| Area | Existing acceptance coverage |
| --- | --- |
| Identity and sessions | Unique email keys/digests, credential storage, owner/session/successor references and rotation-state shape. |
| Accounts and categories | Supported types/currencies, required fields, owner-scoped references and read-only built-ins. |
| Transactions | Amount/date/state/transfer shape, category-kind references and owned account links. |
| Movements and audit | Required fields, exact bounds, owned event/account/actor links and append-only rejection. |
| Financial compatibility | Currency and savings-income restrictions, referenced account changes and atomic migration validation. |

These checks enforce database relationship integrity, not authenticated row-level
access. The API still has no authentication or financial database access; server
read/write authorization and session protocols remain Phase 04 responsibilities.
No SQL migration or access policy is changed by these additional tests.

**Verified locally on 2026-10-25:** both focused tests passed (2 tests, 37.216 s),
followed by the complete suite (36 tests, 735.272 s). An isolated mutation probe
removed the transaction account-owner and account-currency foreign keys and
confirmed that the new transaction test detects the now-accepted invalid write.
All disposable resources were cleaned. The committed migration files were never
changed. These tests exercise existing guards, not a new production behavior;
the mutation probe establishes that the added assertions detect relaxed guards.

### Existing schema behavior coverage

P03.09 adds four `test_audit_*` groups covering minimal audit events, owned actor/
transaction links, field-name-only summaries, append-only rejection and atomic DDL.
See [audit storage](../architecture/transaction-audit-storage.md).
P03.10 adds `test_financial_*` groups for currency/savings rules, compatible account
changes, existing-data validation and two-session FK concurrency; see
[financial constraints](../architecture/financial-constraints.md).

This suite also applies `backend/migrations/*.sql` to fresh disposable databases
and checks identity uniqueness, credential/session ownership, rotation-state
storage and transactional DDL failure. See [identity storage](../architecture/identity-storage.md)
for migration ordering, recovery boundaries and application responsibilities.
The API does not connect to these tables yet.
The same suite checks [account storage](../architecture/account-storage.md),
including supported fields, owner-scoped relationships and failed-DDL rollback.
It also checks [category storage](../architecture/category-storage.md): exact shared
built-ins, stable identifiers, read-only guards and transactional seed/DDL rollback.
It also checks [transaction storage](../architecture/transaction-storage.md): exact
event forms, lifecycle values, owned references and failed-write/DDL rollback.
[Account movement storage](../architecture/account-movement-storage.md) adds signed
postings, append-only guards, compensations, owned references and query indexes.

The [official image documentation](https://github.com/docker-library/docs/blob/master/postgres/README.md)
explains initialization and PostgreSQL 18's `/var/lib/postgresql` volume layout.
Keep the major version at 18; major upgrades and existing data need a separate
upgrade plan. Pulling `postgres:18-bookworm` can update its minor release.

## Continuous integration

The workflow `.github/workflows/ci.yml` runs for every pull request, on pushes
to `main`, through manual dispatch, and weekly on Monday at 07:17 UTC. It uses a
read-only GitHub token and has three jobs:

| Check name | Current behavior |
| --- | --- |
| `Repository / Validate` | Checks required files, UTF-8, LF newlines, final newline, trailing whitespace, and merge markers. |
| `Backend / Rust` | Runs format, Clippy, Rust tests and isolated PostgreSQL migration/infrastructure tests. |
| `Backend / Audit` | Audits the committed Cargo.lock against fresh RustSec advisories and registry yanks. |

`Backend / Rust` runs:

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

`backend/Cargo.lock` is tracked so that `--locked` enforces reproducible
dependency resolution.

The same job then runs `python3 tests/infrastructure/test_postgres_compose.py`
from the repository root, using Docker Compose and disposable local databases.
No database credentials or external mail service are needed in CI.

### Dependency audit

From `backend/`, install the pinned development tool and audit the lockfile:

```bash
cargo install cargo-audit --version 0.22.2 --locked --no-default-features
cargo audit --file Cargo.lock --deny warnings
```

See the [dependency audit policy](DEPENDENCIES.md) for blocking findings, network
requirements, and exception handling. No advisory ignores are configured.

### Required checks

After the first pull request runs CI successfully, configure the branch ruleset
for `main` to require **`Repository / Validate`**. Require
`Backend / Rust` once its real workspace checks pass in the pull request.
Branch ruleset changes are a separate maintainer action.

The workflow intentionally uses no path filters: required checks should run on
every pull request, even when only documentation changes.

## Dependency updates

`.github/dependabot.yml` enables weekly update pull requests for GitHub Actions.
Cargo dependencies are audited by `Backend / Audit`; automated Cargo updates
are not configured yet. Gradle updates remain deferred until that project exists.

## Working conventions

- Use feature branches and pull requests. Do not push directly to `main`.
- Keep documentation, commits, and pull request descriptions in English.
- Never commit `.env` files, credentials, signing keys, or personal financial data.
- Run the repository checks locally before opening a pull request.
- Configure Android and iOS pipelines only when their corresponding projects exist.

## Next milestone

P02.04 initializes the workspace, P02.05 documents Rust tooling, and P02.06 adds
isolated local PostgreSQL. P02.07 adds example configuration and P02.08 adds
process liveness. Backend database access, readiness checks, and financial models
remain future roadmap work.
