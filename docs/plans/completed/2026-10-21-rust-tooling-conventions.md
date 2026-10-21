# Execution plan: Rust tooling conventions

- **Status:** Completed
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.05
- **Tracking issue / PR:** [PR #6](https://github.com/cuenora/cuenora/pull/6) (merged)
- **Owner:** Codex

## Goal and user-visible outcome

Make the existing Rust formatting and linting workflow explicit and verify it.
Assess a task runner without introducing one unless it solves a current need.

## Existing state and evidence

- P02.04 is merged and the two-crate workspace is dependency-free.
- `backend/rust-toolchain.toml` pins Rust 1.99.1 with Rustfmt and Clippy.
- CI already checks formatting, denies Clippy warnings, and runs locked tests.
- Setup instructions already list the commands; no custom formatter/linter
  configuration, Makefile, or justfile exists.
- `.editorconfig` uses four spaces for Rust and LF line endings.

## In scope

- Document default formatting/lint policy and the local formatting command.
- Verify the existing workspace commands and assess the optional task runner.
- Mark only P02.05 after its checks pass.

## Out of scope

- New dependencies, task runners, custom lint sets, CI changes, application code,
  PostgreSQL, and subsequent roadmap tasks.

## Constraints and invariants

Keep the pinned compiler and existing CI checks. Preserve unrelated `.atl/`.
Use English documentation and the existing PR workflow. Remain proportionate:
this task is resumable from the request and diff, so no separate ODD task file
or delegated worker is required.

## Steps

- [x] 1. Inspect current tooling and determine the remaining gap.
- [x] 2. Document formatting/lint conventions and task runner decision.
- [x] 3. Verify commands and record observed results.
- [x] 4. Mark P02.05 complete after successful verification.

## Acceptance criteria

- [x] Setup documents how to format locally and run check-only formatting.
- [x] Rustfmt/Clippy policy is explicit and matches existing CI and compiler.
- [x] The optional task runner has a documented need-based decision.
- [x] Existing workspace checks pass without changing Rust source or CI.

## Verification

Run Cargo commands from backend on pinned Rust 1.99.1.

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `cargo fmt --all -- --check` | Clean formatting | Passed (exit 0) | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | No warnings | Passed (exit 0) | Pass |
| `cargo test --workspace --all-features --locked` | Test harnesses pass | Passed (exit 0) | Pass |
| `cargo build --workspace --locked` | Workspace builds | Passed (exit 0) | Pass |
| `python3 scripts/check_repository.py` (root) | Text conventions pass | Passed (exit 0) | Pass |
| `git diff --check` (root) | No whitespace errors | Passed (exit 0) | Pass |

## Risks, assumptions, and decisions

- Risk: none. Documentation and completion status only; no runtime, toolchain,
  dependency, or CI configuration changes.
- Use Rustfmt defaults from the pinned toolchain; avoid redundant config files.
- Retain Clippy defaults and CI's warnings-as-errors policy. Do not introduce
  blanket lint suppressions or new lint groups without a concrete reason.
- Defer make/just: the existing short Cargo commands are already documented and
  usable directly. A wrapper currently duplicates them and adds maintenance;
  revisit when repeated orchestration makes it useful.
- No meaningful RED test exists for passive documentation. Structural readback
  and existing tool invocations are the proportional verification.

## Completion notes

Verified on Rust 1.99.1 (58cf8f9c3 2026-10-13): formatting, Clippy with
warnings denied, locked workspace tests, and locked workspace build all passed.
The empty unit/doc-test harnesses ran zero tests. Repository validation and
whitespace checks passed. Documentation was read back against actual commands
and configuration. No Rust source, compiler pin, dependencies, or CI changed.

Updated `docs/development/SETUP.md` and only the P02.05 roadmap checkbox.
The verified changes are delivered together on `chore/rust-tooling-conventions`
through PR #6, merged as `42c0e363457d4b5ffb765a419d31343ec647e237`.
Both hosted CI checks passed. This plan is archived after merge.
No P02.06 work was started.
