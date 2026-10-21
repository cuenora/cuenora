# Execution plan: Axum liveness endpoint

- **Status:** Completed
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.08
- **Tracking issue / PR:** [PR #11](https://github.com/cuenora/cuenora/pull/11) (merged)
- **Owner:** Codex

## Goal and user-visible outcome

Run the API locally and serve GET /health/live without infrastructure dependencies.

## Existing state and evidence

- API binary contains only an empty main; domain library remains empty.
- Workspace uses Rust 2024 and compiler 1.99.1, with format/Clippy/test CI checks.
- Axum/Tokio are planned but no Cargo dependencies are declared yet.
- Local PostgreSQL and example config exist; liveness must not depend on them.
- Pre-existing user edits in master-plan.md check P01.01-P01.03 and P02.01-P02.03
  and add blank lines. Preserve these changes; they are outside this task.

## In scope

- Minimal API Axum router, Tokio server bound to 127.0.0.1:3000, and locked dependencies.
- One HTTP-level route test, minimal OpenAPI contract, and run/check instructions.
- Verify behavior and mark only P02.08 after checks pass.

## Out of scope

- Readiness endpoint, database connection, auth, financial models, middleware,
  new crates, deployment, configurable bind addresses, and subsequent roadmap tasks.

## Constraints and invariants

Keep domain free of infrastructure dependencies. Liveness returns no sensitive
information and works without PostgreSQL or environment secrets. Preserve user
changes and .atl. Use the existing pinned compiler and CI commands.

## Steps

- [x] 1. Add minimal dependencies and observe a route test fail with an empty router.
- [x] 2. Implement GET /health/live and local server, keeping the test green.
- [x] 3. Update public documentation and the API contract.
- [x] 4. Verify checks, actual HTTP response and startup failure; independent verification.

## Acceptance criteria

- [x] GET /health/live returns status 200, text/plain content, and exact body OK.
- [x] API listens only on 127.0.0.1:3000 and reports bind errors with nonzero exit.
- [x] Endpoint responds without database/configuration and exposes no diagnostics.
- [x] Domain has no dependencies; dependencies serve concrete runtime/test needs.
- [x] Contract, README, setup instructions and observed verification agree.

## Verification

Run Cargo commands from backend on Rust 1.99.1.

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused route test before implementation | 404 versus expected 200 | Passed (observed) | Pass |
| `cargo fmt --all -- --check` | Formatting passes | Passed (observed) | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | No warnings | Passed (observed) | Pass |
| `cargo test --workspace --all-features --locked` | Route test passes | Passed (observed) | Pass |
| `cargo build --workspace --locked` | Both crates build | Passed (observed) | Pass |
| Real HTTP smoke probe | 200, text/plain, body OK without database | Passed (observed) | Pass |
| Occupied bind port | Second process fails, original keeps responding | Passed (observed) | Pass |
| `python3 scripts/check_repository.py` | Repository conventions pass | Passed (observed) | Pass |
| `git diff --check` | No whitespace errors | Passed (observed) | Pass |

## Risks, assumptions, and decisions

- Risk: item 5 (first HTTP/runtime dependencies and lockfile changes).
- Use Axum 0.8 HTTP/1 and Tokio serving features; Tokio enables macros, networking,
  and its multithread runtime. Tower util is a dev dependency for in-process HTTP tests.
- No shared workspace dependency declarations are needed for one consumer.
- Plain OK body is the minimal liveness response; it is not a database readiness check.
- A small router factory lets the server and HTTP-level test share actual routing.
- Existing API binary has no prior server behavior to preserve; no sibling command
  parser or domain helpers change. Preserve its dependency-free domain crate.
- Sources: https://docs.rs/axum/0.8.9/axum/ and
  https://docs.rs/tower/0.5.3/tower/util/trait.ServiceExt.html.

## Completion notes

RED observed: the HTTP route test compiled and failed with actual 404 versus
expected 200. After registering the route it passed. All four required Cargo
checks passed on Rust 1.99.1, with one API route test and empty domain/doc-test
harnesses. The lockfile resolves Axum 0.8.9, Tokio 1.54.0, and Tower 0.5.3.

A real local server returned HTTP 200, the documented content type and exact OK
body without database variables or credentials. A second process exited nonzero
on the occupied port; the original response remained identical. Only the process
started for this verification was stopped.

Independent verification repeated all four Cargo checks and actual HTTP/bind
failure probes in a fresh scratch copy, including startup with an empty
environment; no blockers found. Parent removed the verifier scratch directory.
Domain stays dependency-free; contract and public documentation were read back.
Repository validation and whitespace checks passed. Rustfmt normalization preceded
verification. The generated lockfile accounts for most changed lines; authored
code/docs remain a single focused work unit.

Native review `review-61dd903ee5f1a9b3` approved all four lenses with no findings.
Its exact acknowledgement completed and consumed the review authority; no code
corrections were required. No dependency vulnerability scan was run in this task.
Implementation commit `255084c` was delivered through PR #11. The maintainer
explicitly authorized `size:exception`: 375 generated lockfile lines account for
most of the diff, and the endpoint, dependencies, test, and docs stay together.
The implementation was merged as `0eda7f0411dc77087eba76de6d7c6c054f75b6a7`.
Both hosted checks, `Repository / Validate` and `Backend / Rust`, passed. Local
`main` was synchronized and the obsolete implementation branch removed after
confirming the tree matched. This plan is archived after merge.

The maintainer subsequently authorized publishing their existing roadmap updates
and reconciling already completed work. P02.09 can now be marked from this task's
existing Rust route test and successful CI, without starting new implementation.
The archival PR also records evidence for other reconciled roadmap checkboxes.
