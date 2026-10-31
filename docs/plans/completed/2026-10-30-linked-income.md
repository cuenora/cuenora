# Execution plan: Normal-account income

- **Status:** Completed
- **Date:** 2026-10-30
- **Roadmap task ID(s):** P05.09
- **Tracking issue / PR:** [PR #79](https://github.com/cuenora/cuenora/pull/79) — merged
- **Owner:** Maintainer and agent
- **Merge evidence:** `cb682ed2bbdf79c0d988dd5ef1d4f579e0314b47` on 2026-10-30; all four hosted checks succeeded.
- **Archived:** 2026-10-31; scoped acceptance criteria verified complete.

## Goal and user-visible outcome

Record income into an owned active normal account with exactly one positive posting.
Full specification and log: [P05.09 work record](../../../odd/tasks/p05-09-linked-income.md).

## Existing state and evidence

P05.08 provides accountless income, atomic audit and permanent retry keys.
Migration 0008 already rejects savings links on inserts and updates.
Account management defines owner→account locking and rejects new postings to archived accounts.

## In scope

Optional account UUID on income creation, ownership/type/currency/active checks,
atomic posting, retries, regression tests, contract and policy updates.

## Out of scope

Edit/void endpoints (P05.13), expenses, reports, verification runtime and publication.

## Constraints and invariants

Preserve old accountless fingerprints and responses; exact money and dates;
verified owner and confirmed profile zone; no partial writes or duplicate movements.

## Steps

- [x] Observe RED, implement and normalize.
- [x] Run focused and workspace checks, isolated PostgreSQL and independent verification.
- [x] Record delivery and native candidate review outcome.

## Acceptance criteria

- [x] Normal-account income posts once, including concurrent retries.
- [x] Savings, foreign, missing, archived and wrong-currency accounts cause no writes.
- [x] Late storage failure rolls back event/movement/audit/key.
- [x] Savings edits/reassignment remain rejected by existing constraints.
- [x] Accountless requests/retries remain compatible; docs/contract agree.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused income PostgreSQL harness | Eleven database cases pass | Eleven passed (12.619s) | Passed |
| Cargo fmt/clippy/workspace tests | Pass | fmt/Clippy passed; 89 passed, 85 ignored | Passed |
| Repository checks | Pass | Repository validator and diff whitespace passed | Passed |
| Independent isolated verification | No change-caused blockers | S1/S2 passed, including baseline→candidate durable retry probes | Passed |

| Full disposable PostgreSQL infrastructure | Pass | 56 passed in 1176.283s on frozen candidate before two extra assertion additions | Passed |

## Risks, assumptions, and decisions

Risk items 2–4: ownership, shared contract and concurrency.
No migration required. Absent/null account remains accountless; a linked UUID must
refer to an owned active normal account in the income currency. Future editing
HTTP behavior remains P05.13; storage compatibility is tested here.

## Completion notes

Implementation and own/independent checks passed. Full infrastructure ran in an
isolated scratch candidate; baseline-created durable retry records replayed with
identical responses and persisted hashes. Adopted permanent changed-destination
conflict and exact foreign/missing response comparisons, then reran focused tests
and Clippy. Initial test-only E0716 was fixed before all successful verification.
Implementation commit: `a1bcfa0`. Single-PR size exception approved; publication
and PR requested. The maintainer chose to skip native review for this candidate;
the exact native response confirmed `declined_this_candidate`. Future reviews
remain enabled. The maintainer authorized configured SSH/GitHub CLI access; the feature branch was published and subsequently merged through PR #79, as recorded above.

Rollback boundary: revert this work unit to remove linked-income input/postings and
its tests/contracts/docs together; existing P05.08 accountless behavior and schema
remain intact. No migration or stored-data rewrite is included.
