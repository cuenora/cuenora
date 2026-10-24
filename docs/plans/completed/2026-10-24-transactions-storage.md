# Execution plan: Transaction event storage

- **Status:** Completed and merged
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.07
- **Tracking issue / PR:** [PR #33](https://github.com/cuenora/cuenora/pull/33)
- **Owner:** Agent

## Goal and user-visible outcome

Add reproducible transaction-event storage, with `posted` and `voided` states. This is not a posting API and does not calculate balances.

## Existing state and evidence

Migrations `0001`–`0004` provide identity, account and global-category storage; the existing suite has 15 tests. See the [task specification](../../../odd/tasks/p03-07-transactions-storage.md) for approved scope and the confirmed lifecycle decision.

## In scope

One transaction migration, owned relationships, exact amounts, dates, basic row guards, isolated PostgreSQL tests and storage documentation.

## Out of scope

Movements, audit, authentication, APIs, correction protocols, full P03.10 account/type/currency guards and indexes, quoted-rate policy (P06.01), and transfer idempotency (P06.07).

## Constraints and invariants

Preserve existing migrations and financial data. Reuse transactional migration/snapshot helpers. No floats, balance column, runtime dependency, CI change or personal data. NULL account references represent accountless income/expense, not implicit cash accounts.

## Steps

- [x] 1. Observe focused RED; implement migration and GREEN.
- [x] 2. Run full suite and independent scratch verification; record proof.
- [x] 3. Commit the coherent work unit and follow native assessment/consent.

## Acceptance criteria

- [x] Exact amounts, supported event forms, two states and civil/audit dates round-trip.
- [x] Invalid rows and cross-owner/category relationships fail without changing stored data.
- [x] Reapply and late-DDL failure preserve prior schema/data; old tests pass.
- [x] Deferred application/roadmap responsibilities remain explicit.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused transaction tests | RED before migration, GREEN after | Five missing-migration failures (31.403s), then five pass (105.942s) | Pass |
| `python3 tests/infrastructure/test_postgres_compose.py` | Old and new tests pass | Baseline 15 pass (170.593s), final candidate 20 pass (282.377s) | Pass |
| `python3 scripts/check_repository.py` | Repository conventions pass | Passed; 120 files | Pass |
| Independent probes and parent spot check | S1–S4 satisfied | Two probes pass (39.017s), no blockers; parent repeated both successfully (38.479s) | Pass |
| Existing category mutation test | Accept approved early FK `0A000` or guard `55000`; preserve data | Original failure reproduced (12.821s), then approved expectation passed (15.378s) | Pass |
| Native candidate review after explicit consent | Bound review completes | Four lenses, zero findings; approved and exact acknowledgement confirmed authority burned | Complete |

## Risks, assumptions, and decisions

Risk: items 1 (migration) and 2 (ownership). SQL authorization and posting invariants are not implemented by foreign keys or a state field. Forecast ~380 authored lines; delivery strategy `ask-on-risk`.

## Completion notes

Migration and five new groups implemented; full suite and independent verification pass. The user approved the category TRUNCATE error-contract change: native FK preflight may reject with `0A000` before the read-only guard's `55000`. Differential probes confirm no other mutation output changes and no stored-data changes on rejection. Shared helpers and all earlier migrations remain unchanged. Local Cargo checks were not applicable (no Rust/dependency changes); repository validation ran in the live checkout because scratch archives lack `.git`.

Work-unit commit: `d81edbdd7d8bc5bc7086f3183d6e6b7127aec4df` (346 additions, 8 deletions); verification bookkeeping: `5075f4e8581666f2659a75e7588726c090e48212`. After explicit user consent, native review `review-04bbfc54bbd25a4a` approved this slice with zero findings across four lenses. Exact acknowledgement confirmed burned authority at revision `sha256:e2ac4c0481dbe4a585a5c8da11092f67177cb25c4f4512c4f81b0dabf8c35c61`; no correction was required.

Rollback boundary is this transaction storage slice and its tests/docs, not existing identity/account/category data. [PR #33](https://github.com/cuenora/cuenora/pull/33) merged on 2026-10-24 at 20:57:40 UTC as `cc049a183abdd4ad172ca2fcf955e803e521ff92`, with all four PR checks successful. The merged tree matches the verified published head `2bea50ab4961afaa4adc5d1e71652974711bb249`. P03.07 is complete and this plan is archived after merge. P03.08 account movements and the remaining Phase 03 work are not completed by this archive.
