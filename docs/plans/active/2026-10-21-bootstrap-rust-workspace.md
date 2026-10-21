# Execution plan: Bootstrap the Rust backend workspace

- **Status:** Active (implementation verified; awaiting PR and merge)
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.04
- **Tracking issue / PR:** Not created
- **Owner:** Codex

## Goal and user-visible outcome

Provide a buildable Rust 2024 Cargo workspace with a domain library and an API
binary, without implementing HTTP or financial behavior.

## Existing state and evidence

- `backend/rust-toolchain.toml` pins Rust 1.99.1 with Rustfmt and Clippy.
- No Cargo manifest, source files, or lockfile existed before this task.
- `.github/workflows/ci.yml` already runs format, Clippy, and locked tests when
  `backend/Cargo.toml` exists. No CI change is necessary.
- `.gitignore` excludes build output and permits tracking Cargo.lock.
- README and setup instructions still describe initialization as future work.
- Local mise shims select Rust 1.98.1; verification must explicitly select 1.99.1.
- Existing untracked `.atl/` is unrelated and must remain untouched.

## In scope

- Workspace, two crates, shared package metadata, and generated Cargo.lock.
- Documentation updates and verification of the existing CI command sequence.

## Out of scope

- HTTP frameworks/endpoints, authentication, database access, financial models,
  migrations, mobile applications, additional crates, and external dependencies.
- Other roadmap tasks and branch protection configuration.

## Constraints and invariants

Follow root AGENTS.md. Keep domain independent of infrastructure. Use edition
2024, resolver 3, and the existing pinned toolchain. No financial or user data is
handled. Work on a feature branch and retain this plan in active until merge.

## Steps

- [x] 1. Add the virtual workspace and two dependency-free member crates.
- [x] 2. Generate Cargo.lock and update README and development instructions.
- [x] 3. Run all requested Cargo commands and repository validation.
- [x] 4. Record results, mark only P02.04 complete after successful verification,
      and commit the focused change including Cargo.lock.

## Acceptance criteria

- [x] Cargo metadata reports exactly cuenora-domain (library) and cuenora-api
      (binary), with edition 2024 and resolver 3.
- [x] Shared version, edition, license, repository, and publish policy are inherited.
- [x] Both crates have no dependencies or placeholder business logic.
- [x] All four requested Cargo commands pass on Rust 1.99.1.
- [x] Existing CI commands work with the new workspace; docs describe actual state.
- [x] Cargo.lock is committed; no subsequent roadmap task is started.

## Verification

Run from backend with the pinned Rust 1.99.1 toolchain.

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | Clean formatting | Passed (exit 0) | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | No warnings | Passed (exit 0) | Pass |
| `cargo test --workspace --all-features --locked` | Successful test and doc-test harnesses | Passed (exit 0) | Pass |
| `cargo build --workspace --locked` | Both crates build | Passed (exit 0) | Pass |
| `cargo metadata --no-deps --format-version 1 --locked` | Exactly two dependency-free members | Passed (exit 0) | Pass |
| `python3 scripts/check_repository.py` (root) | Repository conventions pass | Passed (exit 0) | Pass |
| `git diff --check` (root) | No whitespace errors | Passed (exit 0) | Pass |

## Risks, assumptions, and decisions

- Risk: item 5 (new Cargo build environment and lockfile; existing CI unchanged).
- Use `backend/crates/domain` and `backend/crates/api` from the planned layout.
- Keep library source as crate documentation and binary as an empty main function.
  No runtime behavior exists to justify artificial unit tests or RED/GREEN tests;
  compilation, Cargo metadata, linting, and test harnesses verify the bootstrap.
- Share package metadata without speculative dependency declarations or unused
  links between crates. Version 0.1.0 is unreleased; publishing is disabled.
- Rollback boundary: new manifests, sources, lockfile, and associated docs only.
- Remote push/PR and hosted CI remain pending authorized remote access.

## Completion notes

All four requested Cargo commands passed on rustc 1.99.1
(58cf8f9c3 2026-10-13). Unit and doc-test harnesses ran successfully with zero
tests, as expected for empty crates. Cargo metadata confirmed exactly two members,
no dependencies, and inherited metadata. Repository validation passed; whitespace
checks passed. Existing CI format/Clippy/test commands were executed locally and
passed; hosted GitHub Actions has not run for this branch. No CI edits were needed.

Files changed: workspace and member manifests, library and binary entry files,
generated Cargo.lock, toolchain comment, README, setup guide, this execution plan,
and the P02.04 checkbox. No other roadmap checkbox is changed.

Independent verification passed all four Cargo commands in an isolated fresh
scratch copy and found no change-caused blockers. Native review is pending.
Cargo.lock is included in the focused commit. Remote push and PR creation remain pending
explicit authorization of the destination and authenticated session. Keep this
plan in active until merge. No next roadmap task was started.
