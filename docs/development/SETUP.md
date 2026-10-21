# Development environment

The Rust backend workspace is initialized with two dependency-free crates:
`cuenora-domain` at `backend/crates/domain` and `cuenora-api` at
`backend/crates/api`. The library has no domain models; the binary exits without
starting an HTTP server. The application is not implemented yet.

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

The test harnesses currently contain no tests; business behavior will be tested
when it is introduced.

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

The [official image documentation](https://github.com/docker-library/docs/blob/master/postgres/README.md)
explains initialization and PostgreSQL 18's `/var/lib/postgresql` volume layout.
Keep the major version at 18; major upgrades and existing data need a separate
upgrade plan. Pulling `postgres:18-bookworm` can update its minor release.

## Continuous integration

The workflow `.github/workflows/ci.yml` runs for every pull request, on pushes
to `main`, and through manual dispatch. It uses a read-only GitHub token and
has two jobs:

| Check name | Current behavior |
| --- | --- |
| `Repository / Validate` | Checks required files, UTF-8, LF newlines, final newline, trailing whitespace, and merge markers. |
| `Backend / Rust` | Runs format, Clippy, and test harnesses for the initialized workspace. |

`Backend / Rust` runs:

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

`backend/Cargo.lock` is tracked so that `--locked` enforces reproducible
dependency resolution.

### Required checks

After the first pull request runs CI successfully, configure the branch ruleset
for `main` to require **`Repository / Validate`**. Require
`Backend / Rust` once its real workspace checks pass in the pull request.
Branch ruleset changes are a separate maintainer action.

The workflow intentionally uses no path filters: required checks should run on
every pull request, even when only documentation changes.

## Dependency updates

`.github/dependabot.yml` enables weekly update pull requests for GitHub Actions.
Cargo and Gradle dependencies will be added to Dependabot after the respective
projects have been initialized.

## Working conventions

- Use feature branches and pull requests. Do not push directly to `main`.
- Keep documentation, commits, and pull request descriptions in English.
- Never commit `.env` files, credentials, signing keys, or personal financial data.
- Run the repository checks locally before opening a pull request.
- Configure Android and iOS pipelines only when their corresponding projects exist.

## Next milestone

P02.04 initializes the workspace, P02.05 documents Rust tooling, and P02.06 adds
isolated local PostgreSQL. Backend database access, HTTP health endpoints, and
financial models remain future roadmap tasks.
