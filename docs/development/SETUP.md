# Development environment

This document describes the initial developer tooling for Cuenora. The application
and the Rust backend have **not** been implemented yet.

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
   rustc --version
   cargo --version
   cargo clippy --version
   cargo fmt --version
   python3 scripts/check_repository.py
   ```

The development container image is based on Debian Bookworm and contains Rust
utilities. The Rust toolchain is currently set to `stable` in
`backend/rust-toolchain.toml`; it can be pinned to an exact version after the
first Cargo workspace has been created.

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

There is no Cargo manifest yet. Cargo build and test commands are intentionally
not available until `backend/Cargo.toml` is introduced.

## Continuous integration

The workflow `.github/workflows/ci.yml` runs for every pull request, on pushes
to `main`, and through manual dispatch. It uses a read-only GitHub token and
has two jobs:

| Check name | Current behavior |
| --- | --- |
| `Repository / Validate` | Checks required files, UTF-8, LF newlines, final newline, trailing whitespace, and merge markers. |
| `Backend / Rust` | Provides a placeholder until `backend/Cargo.toml` exists; afterwards runs format, Clippy and unit tests. |

When the backend workspace exists, `Backend / Rust` runs:

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

Commit `backend/Cargo.lock` when the backend is initialized so that `--locked`
can enforce reproducible dependency resolution.

### Required checks

After the first pull request runs CI successfully, configure the branch ruleset
for `main` to require **`Repository / Validate`**. Do **not** require
`Backend / Rust` until there is a real Cargo workspace and its checks pass.

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

Initialize the Rust Cargo workspace, add a minimal HTTP API with a health endpoint,
and activate real backend build, formatting, linting, and unit-test checks.
