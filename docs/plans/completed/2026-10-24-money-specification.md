# Execution plan: Exact money specification

- **Status:** Completed — PR #24 merged and all four hosted checks passed
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.01–P03.02
- **Tracking issue / PR:** [PR #24](https://github.com/cuenora/cuenora/pull/24)
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Implement the specified `Money` and `Currency` value types using strict TDD.
Preserve recorded bank amounts exactly, with checked arithmetic and quantization.

## Existing state and evidence at specification start

- `backend/crates/domain/src/lib.rs` contains only module documentation.
- The domain manifest has no dependencies; the API exposes only liveness.
- SEK, EUR and USD are accepted MVP currencies. Their ISO 4217 minor-unit scale
  is two, verified against the SIX list published on 2026-09-17.
- The maintainer approved two-decimal recorded amounts, negative balances,
  half-even calculated-result rounding and technical-only amount bounds.

## In scope

- A reusable money specification and an ADR explaining the representation.
- Exact examples for parsing, formatting, arithmetic, bounds and rounding.
- Links and planning text that distinguish specification from implementation.
- Public domain value types and tests covering all money acceptance cases.

## Out of scope

New dependencies, migrations, HTTP endpoints, mobile behavior,
date policies, bank integration and interest or exchange-rate calculation engines.

## Constraints and invariants

No binary floats, implicit currency conversion, silent rounding of recorded
inputs or wrapping/saturating arithmetic. Domain calculations remain independent
of HTTP, persistence, clocks and UI. Preserve untracked `.atl/` content.

## Steps

- [x] 1. Inspect the roadmap, domain crate and accepted product policies.
- [x] 2. Specify representation, exact bounds, rounding and acceptance cases.
- [x] 3. Record the architectural choice and align documentation links.
- [x] 4. Check numerical examples, links and repository formatting.
- [x] 5. Write public-interface tests for C01, M01–M07, A01–A06 and R01–R06; observe RED.
- [x] 6. Implement P03.01–P03.02 inline; observe GREEN, then refactor.
- [x] 7. Run pinned-toolchain formatting, Clippy and workspace tests; update status.

## Acceptance criteria

- [x] Product decisions are distinct from engineering choices and deferred policies.
- [x] Valid, invalid and boundary examples have deterministic expected results.
- [x] Documentation distinguishes implemented domain types from pending application features.
- [x] P03.01 and P03.02 are checked only after implementation and observed acceptance tests.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Independent Python integer/Decimal calculations | Bounds and half-even cases agree with the specification | Range, 11 decimal examples and 11 ratio cases passed | Passed |
| Relative Markdown link and table checks for changed files | All local file targets exist; table columns are consistent | Passed across all eight changed documents | Passed |
| `python3 scripts/check_repository.py` | Repository text validation passes | Passed; 94 Git-visible files examined | Passed |
| `git diff --check` | No whitespace errors | Passed | Passed |
| Rust RED/GREEN tests in the initial specification cycle | Deferred to implementation | No code changed during that cycle; implementation evidence follows below | Not applicable to initial cycle |

### Implementation verification (from `backend/`)

| Command | Observed result | Status |
| --- | --- | --- |
| `rustup run 1.99.1 cargo test -p cuenora-domain --test money --locked` (tests only) | E0432, public types absent; exit 101 | Expected RED |
| Same command with `-- --quiet`, after adding unimplemented signatures only | 0 passed, 20 failed; exit 101 | Expected runtime RED |
| `rustup run 1.99.1 cargo test -p cuenora-domain --test money --locked` (implemented) | 20 passed, 0 failed; exit 0 | GREEN |
| `rustup run 1.99.1 cargo clippy --workspace --all-targets --all-features -- -D warnings` | First run rejected manual parity check; replaced it with `is_multiple_of`, rerun passed | Passed after correction |
| `rustup run 1.99.1 cargo test --workspace --all-features --locked` | 20 domain tests and 1 existing API test passed; doc-test harness had 0 tests | Passed |
| `rustup run 1.99.1 cargo fmt --all -- --check` | Passed after normalization with pinned Rustfmt | Passed |

## Risks, assumptions, and decisions

- See [ADR-002](../../architecture/decisions/002-exact-money.md) and the
  [money specification](../../product/money.md).
- This documents a Cuenora default, not a universal bank interest convention.
- The proposed schema's decimal column choice remains open until migrations;
  it must preserve this range and reject excessive input precision before storage.

## Completion notes

The initial specification cycle was documentation only, on
`docs/p03-money-specification`. The subsequent authorized implementation cycle is
recorded below. The maintainer
subsequently requested a PR for these changes; no dependencies were added.

The first ad hoc structural probe incorrectly assumed every table had three
columns; correcting the probe to use each table's header made it pass. No document
defect was hidden or suppressed. All final document checks passed.

## Implementation cycle log

- **Request (2026-10-24):** "Excelente, implementa P03.01-P03.02 en cuenora-domain, usando la especificación acordada para ello inicia un nuevo ciclo ODD y hazlo como lo haz descrito en TDD estricto, creando primenro los test que fallen tal como lo descrives"
- **Route:** Small direct inline unit; fully specified and resumable from the
  request, specification and diff. No shared Money callers, persistence,
  dependencies or other high-risk items. Risk: none.
- **Runner:** `rustup run 1.99.1 cargo ...` from `backend/`; the ambient
  `RUSTUP_TOOLCHAIN` overrides the project pin with 1.98.1, so invoke the installed
  pinned compiler explicitly. No toolchain or environment configuration changes.
- **Baseline:** Domain library has no exports or tests. The existing API liveness
  test remains the workspace regression check; no domain CLI differential applies.
- **RED (tests first):** `rustup run 1.99.1 cargo test -p cuenora-domain --test money --locked`
  exited 101 with E0432: the three public types did not exist. After adding only
  type declarations and unimplemented method signatures, the same command with
  `-- --quiet` compiled and reported **0 passed, 20 failed**, exit 101. No monetary
  behavior was implemented before either observation.
- **GREEN:** Implemented currency parsing/formatting, exact recorded parsing,
  canonical formatting, symmetric bounds, checked arithmetic and rational
  half-even quantization. The focused acceptance command passed all 20 groups.
- **Refactor:** Kept construction validation centralized; formatting normalized
  source and tests. Clippy requested `is_multiple_of` for midpoint parity; made
  that equivalent change, then observed workspace tests remaining green.
- **Scope:** All C01, M01–M07, A01–A06 and R01–R06 cases are covered through the
  public interface, with single-currency cases repeated for SEK/EUR/USD. No
  existing exported domain behavior was replaced. The unchanged API liveness
  test passed in the full workspace run. No CLI differential check applies.
- **Final repository checks:** `python3 scripts/check_repository.py` passed
  (96 Git-visible files); `git diff --check` and changed-document relative-link
  checks passed. No unimplemented bodies or binary-float types remain in the
  money implementation. Rust 1.99.1 was used without changing the ambient override.
- **Native assessment:** Review mode was on. Initial assessment required an
  explicit declaration of untracked files; the canonical inventory identified
  five authorized new source/test/document files and the unrelated `.atl/`
  files. Assessment with those five intended paths and the inventory digest
  succeeded: `risk: medium`, `review_due: false`, `review_due_reason: under_budget`.
  No review transaction was started, no consent was requested and no approval
  receipt is claimed. Existing functional proof stands; no high-risk item calls
  for an independent verifier. Assessment and scope are informational.
- **Delivery request (2026-10-24):** "Primero crea la PR para estos cambios"
  The delivery branch is `feat/exact-money`, targeting `main`. Preserve `.atl/`
  outside the commit. Publish using the previously authorized GitHub CLI HTTPS
  session for `cuenora/cuenora`; do not merge. The complete specification,
  implementation and test diff exceeds 400 lines.
- **Size approval (2026-10-24):** "Sí, lo acepto" — the maintainer explicitly
  accepted `size:exception` for one PR keeping the specification, code and tests
  together. The implementation commit `efa1da2` contains 924 changed lines.
- **Publication:** [PR #24](https://github.com/cuenora/cuenora/pull/24) targets
  `main` from `feat/exact-money`. No merge is included in the authorized scope.
- **Merged by maintainer:** PR #24 merged on 2026-10-24 at 10:04:57 UTC as
  `58e6e315a592190c9b2968f8188c4d7001dfb024`. Repository / Validate,
  Backend / Rust, Backend / Audit and Backend / Dependency Review all passed.
  Local `main` was fast-forwarded to the merge; its tree matches `d5b9647` exactly.
  Archived after this verification. P03.03 and the remaining Phase 03 tasks stay pending.
