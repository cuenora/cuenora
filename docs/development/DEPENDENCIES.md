# Rust dependency audit policy

Audit the backend's committed dependency resolution against the RustSec Advisory
Database before merging changes. CI runs the same audit on pull requests, main
pushes, manual runs, and weekly on Monday at 07:17 UTC. The weekly run also repeats
repository validation and the existing Rust quality checks.

## Run locally

Use the toolchain pinned in `backend/rust-toolchain.toml`. From `backend/`:

```bash
cargo install cargo-audit --version 0.22.2 --locked --no-default-features
cargo audit --file Cargo.lock --deny warnings
```

Installation pins the development tool and its own locked dependencies; it does
not add cargo-audit to the application workspace. Optional binary scanning and
automatic fixes are not enabled. The audit explicitly names `Cargo.lock` so a
missing lockfile fails instead of generating a new dependency resolution.

The real audit needs network access to fetch current RustSec advisories and check
registry yanks. Local caches are outside the repository. Do not use `--no-fetch`,
`--stale`, or `--no-yanked` to make CI pass. Tool installation, advisory/index
retrieval, missing input, or parsing failures must produce a failed job.

## Scope and blocking findings

Audit registry dependencies recorded in `backend/Cargo.lock`, including transitive
and development dependencies. Do not filter by platform or minimum severity.
Known vulnerabilities block the check. `--deny warnings` also blocks the warning
categories supported by this tool version, including unmaintained, unsound, and
yanked dependencies and applicable tool-self advisories. Informational notices
are reported but are not universally denied by this flag.

This is a dependency advisory check, not a source-code audit, license audit,
container-image scan, binary scan, or proof that no unknown vulnerabilities exist.
It does not replace formatting, Clippy, tests, or review. GitHub Actions updates
remain handled by the existing Dependabot configuration; Cargo update automation
is separate work. GitHub Dependency Review complements the full lockfile audit
by checking dependency changes introduced in pull requests.

## Respond to a failure

Read the advisory and prefer a compatible dependency update or replacement.
Update manifests/lockfile intentionally, then run the documented format, lint,
test, build, and audit commands. Never run `cargo audit fix` automatically in CI
or bypass errors with `continue-on-error`, `|| true`, broad ignores, or severity
thresholds. A network outage is a failed audit, not a clean result.

This repository currently has no advisory exceptions. Any future narrow exception
requires explicit maintainer approval and a tracked rationale that identifies the
advisory, affected package/version, applicability, remediation owner, expiry, and
review date. Keep private vulnerability details out of public issues; follow
[SECURITY.md](../../SECURITY.md). Do not silently introduce an ignore list.

## CI behavior

`Backend / Audit` uses a GitHub-hosted Ubuntu runner, read-only repository
permissions, and checkout without persisted credentials. It invokes the pinned
CLI directly and neither opens issues nor posts comments or applies fixes.
A missing workspace is no longer treated as a successful skipped Rust check.
The separate `Backend / Dependency Review` check runs only on pull requests using
`actions/dependency-review-action@v5` with `contents: read`. It blocks introduced
known vulnerabilities at low severity or above across runtime, development and
unknown scopes, posts no PR comments, and disables license checks. API, input and
parsing errors fail the check. GitHub snapshot warnings can indicate incomplete
comparison coverage; the action reports these warnings but does not fail solely
because of them. Review such warnings before merging. It uses GitHub's dependency
graph and comparison API. No checkout or execution of PR code is needed.

The dependency graph must be enabled in repository settings. Verified `main` rules require PRs and all four checks: `Repository / Validate`,
`Backend / Rust`, `Backend / Audit`, and `Backend / Dependency Review`. The
maintainer configured the rules; the dependency-review check passed on PR #15
and rejected a known vulnerable dependency on the never-merged test PR #16.

## Tool references

- [RustSec cargo-audit](https://github.com/rustsec/rustsec/tree/cargo-audit/v0.22.2/cargo-audit)
- [Upstream configuration](https://github.com/rustsec/rustsec/blob/cargo-audit/v0.22.2/cargo-audit/src/config.rs)
- [Explicit lockfile behavior](https://github.com/rustsec/rustsec/blob/cargo-audit/v0.22.2/cargo-audit/src/lockfile.rs)
