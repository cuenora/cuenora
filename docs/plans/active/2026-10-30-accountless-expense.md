# Execution plan: Accountless expense

- **Status:** Active
- **Date:** 2026-10-30
- **Roadmap task ID(s):** P05.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Record an exact, dated expense without an account or balance effect.
Specification and evidence: [work record](../../../odd/tasks/p05-10-accountless-expense.md).

## Existing state and evidence

Income creation provides strict transport, verified-owner/date gates and permanent
retries. Transaction storage already supports accountless expense events.

## In scope

Accountless expense POST, compatible categories, independent permanent keys,
minimal atomic audit, migration, regression tests and API documentation.

## Out of scope

Linked expenses, edit/void, reporting endpoints, email-verification runtime and publication.

## Constraints and invariants

Exact positive money; explicit past/today date in confirmed zone; verified owner;
no movements or balance changes; immutable owner-scoped retry keys; no partial writes.

## Steps

- [x] Observe RED and implement GREEN.
- [x] Run own checks and independent verification.
- [x] Record delivery strategy, commit and native review choice.

## Acceptance criteria

- [x] Exact expense stored with minimal audit and no movement.
- [x] Retry, concurrency, namespace isolation and rollback proven.
- [x] Rejections preserve financial state; existing income behavior unchanged.
- [x] Documentation and contracts match the implementation.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused expense/income PostgreSQL harness | Pass | Expense five and income eleven database cases passed; migration upgrade passed | Passed |
| Cargo fmt/Clippy/workspace tests | Pass | fmt and Clippy passed; 90 passed, 90 ignored | Passed |
| Repository and independent checks | Pass | Validator/whitespace passed; S1/S2 passed, including durable baseline-income retries | Passed |
| Full PostgreSQL infrastructure | Pass | 57/58 passed in 1155.020s; schema expectation omitted new table. Corrected and exact failed test passed independently and locally; no full rerun | Scoped correction passed |

## Risks, assumptions, and decisions

Risk items 1–4: new permanent-key schema, ownership, shared transport and concurrency.
See [ADR-030](../../architecture/decisions/030-accountless-expense-retries.md).

## Completion notes

Implementation `c3c8e1c` and own/independent verification passed. The maintainer
skipped native review for this candidate only; future reviews remain enabled.
Publication remains pending. Maintainer approved a single-PR
size exception for this coherent financial behavior. Initial RED: expense creation
returned 400 instead of 201; retry table absent. One unit-test expectation used
an invalid synthetic bearer but expected backend unavailability; corrected to
the existing 401 contract before successful final checks. Independent probes were
added as regressions. Full-suite failure was a missing expected-table entry,
not a storage failure; corrected once and rechecked without another full sweep.
Rollback requires a reviewed forward
migration if expense retry records exist; never drop durable keys casually.
