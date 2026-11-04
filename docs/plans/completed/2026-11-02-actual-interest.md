# Execution plan: Record actual interest

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P08.10
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/101

## Goal and user-visible outcome

Explicitly confirmed interest creates one income event and one credit in the owned receiving account through `POST /v1/transactions` with `kind: interest_income` (maintainer approved).

## Existing state and evidence

The transaction schema, listing and correction paths already recognize interest income.
Creation rejects it today; ordinary income must continue rejecting savings receivers.

## In scope

- Positive actual amounts, explicit dates and required normal/savings receiver.
- Reuse verified ownership, confirmed zone, owner-first locks and atomic event/movement/audit/key writes.
- Preserve existing income fingerprints; interest uses a distinct fingerprint in the income key namespace.
- HTTP regression tests, OpenAPI and product documentation.

## Out of scope

Forecast confirmation/reconciliation, automatic payout scheduling, new migrations, remote operations and P08.11/P08.12.

## Constraints and invariants

Actual amounts need not match forecasts and need no configured interest terms.
Retries of the same key never post again; different keys represent separate confirmations.
Category remains optional and income-compatible, as in existing income creation.

## Steps

- [x] Observe failing actual-interest HTTP regression.
- [x] Implement interest-specific creation without changing ordinary income/expense behavior.
- [x] Run isolated PostgreSQL tests, workspace checks and independent verification.

## Acceptance criteria

- [x] Both owned receiver types receive exactly one credit under retries/concurrency.
- [x] Invalid receiver, ownership, currency, dates and identity reject without financial writes.
- [x] Existing ordinary income restrictions and replay behavior remain unchanged.

## Verification

- RED: actual-interest HTTP creation rejected with 400 before implementation.
- GREEN: disposable PostgreSQL income target: 13 passed; expense: 7; transfer: 19; mutations: 10.
- Pinned Cargo workspace: 131 passed, 194 ignored database cases (not counted as passed).
- Formatting, Clippy with warnings denied, repository checker and diff whitespace checks passed.
- Independent archived candidate: concurrent receivers, distinct keys, invalid/future dates and injected audit-failure rollback passed. All 12 normalized HTTP baseline/candidate parser-error responses matched. Baseline negative control rejected the new interest path as expected.
- Separate baseline/candidate Cargo caches were required to avoid archived-source artifact reuse.
- RDD is globally disabled; no native review was started.

## Risks, assumptions, and decisions

Risk: items 2, 3 and 4 (ownership, shared creation path and serialized retries).
No source migration is necessary. Keep untracked `.atl/` unchanged.

## Completion notes

Implemented through the existing income posting boundary with a distinct interest fingerprint; old income fingerprints remain unchanged. Added OpenAPI schemas and documented confirmation/retry semantics. Independent rollback probes were retained as a public HTTP regression; the final 13-test isolated income rerun and Clippy passed after retaining the probe. No migration or dependency change. Delivered with P08.11/P08.12 in one maintainer-authorized PR without an issue and with `size:exception`.

Remaining limitation: duplicate confirmations under different keys cannot be detected automatically. P08.11/P08.12 remain separate follow-up tasks.

Delivery branch: `feat/p08-10-12-actual-interest-verification`. Maintainer explicitly authorized SSH push and the current GitHub CLI session for one PR containing P08.10–P08.12.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `a81e3f0 feat(savings): record actual interest and verify forecasts (P08.10–P08.12) (#101)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
