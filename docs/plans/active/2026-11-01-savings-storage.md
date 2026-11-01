# Execution plan: Savings configuration storage

- **Status:** Locally verified; publication authorized
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P08.02
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Persist optional interest configuration and exact effective-dated nominal rates;
no API or financial operation is exposed.

## Existing state and evidence

P08.01 defines forecast policies in `docs/product/savings.md`. Migration 0008
already provides the composite account/owner/type key. Migration 0017 and the
Compose harness establish native constraints and immutable-history patterns.

## In scope

- Add two independent effective-dated streams, exact rate validation and tests.
- Preserve owner/type references and existing records; create no postings.

## Out of scope

Configuration API, calculator, rate correction workflow, payout transitions,
actual interest recording, remote operations and publication.

## Constraints and invariants

Use exact nonnegative percentages with at most six fractional places, no implicit
rounding. Preserve history, optional configuration and authenticated ownership
boundaries; configuration presence never grants posting authority.

## Steps

- [x] 1. Observe missing-storage RED and add migration.
- [x] 2. Verify isolated PostgreSQL constraints, upgrade and schema catalog.
- [x] 3. Run regression checks and independent verification.

## Acceptance criteria

- [x] Owner/type constraints and effective-date uniqueness hold in PostgreSQL.
- [x] Excess precision and unsupported terms fail without mutation.
- [x] Existing records are unchanged and no financial rows are created.
- [x] Documentation distinguishes storage from future runtime.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| New focused PostgreSQL storage test before migration | Missing-table failure | Failed at first terms insert | RED |
| Storage, populated upgrade and fresh schema checks | Pass | All three passed; independent verifier repeated all three | Passed |
| Infrastructure regression suite | Pass | 76/77 passed; sole failure was old SQLSTATE expectation loaded before correction; corrected focused test passed | Failure resolved by focused rerun |
| Independent S1–S2 verifier | No change-caused blockers | Both PASS; separate reverse-reference/snapshot probe passed | Passed |
| Repository check, fmt, Clippy, Cargo tests | Pass | Passed; 173 database tests ignored by plain Cargo and exercised via infrastructure groups | Passed |

## Risks, assumptions, and decisions

Risk: items 1 and 2 (persisted schema and ownership guards). Two independent
streams preserve existing rates across cadence changes. Updates/deletes/truncates
are prohibited; no invented terms or default capitalization. P08.01 changes were
already uncommitted before this task and must not be silently committed with it.

## Completion notes

Verified locally. No source runtime changes; no API/calculator enabled. Full suite
ran once against pre-correction loaded test code, then its single failure was
corrected (23503 expectation to existing reverse-guard 23001) and the focused
test passed. Independent verifier passed all focused checks and additional
reverse-reference probes in an outside scratch copy. RDD disabled globally.
P08.01 documentation remains uncommitted; maintainer authorized one combined PR without an issue, SSH push and current gh session.
Work log: `odd/tasks/p08-02-savings-storage.md`.
