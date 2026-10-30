# Execution plan: Linked expense

- **Status:** Active
- **Date:** 2026-10-30
- **Roadmap task ID(s):** P05.11
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Record an expense against an owned active normal or savings account with one negative posting. Full specification: [work record](../../../odd/tasks/p05-11-linked-expense.md).

## Existing state and evidence

P05.10 provides expense creation and independent permanent retries. Existing storage supports expense links and signed movements. Linked income provides owner→account locking; approved policy permits negative balances.

## In scope

Optional account UUID, ownership/currency/active checks, atomic negative posting, compatible retries, tests and API documentation.

## Out of scope

Editing, voiding, reports, transfers, verification runtime, migrations and publication.

## Constraints and invariants

Preserve accountless fingerprints and responses, exact money/date validation and existing income behavior. Rejected requests and late failures leave financial state unchanged.

## Steps

- [x] Observe RED, implement GREEN and normalize.
- [x] Run own checks and independent isolated verification.
- [ ] Record delivery and native candidate review outcome.

## Acceptance criteria

- [x] One negative posting, including concurrent retries and overdraft.
- [x] Foreign/missing, archived and wrong-currency accounts rejected without writes.
- [x] Late failure rolls back event, movement, audit and key.
- [x] Existing accountless retries and income behavior preserved.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Isolated expense PostgreSQL harness | Linked and accountless cases pass | 7 expense and 11 income cases passed | Passed |
| Cargo fmt, Clippy, workspace tests | Pass | fmt/Clippy passed; Cargo 90 passed, 92 ignored | Passed |
| Repository checks and independent probes | Pass | Repository check; S1/S2 independent probes passed | Passed |

## Risks, assumptions, and decisions

Risk items 2/3/4: ownership, shared transport/API contract and concurrent posting. No stored-data rewrite. Reverting this unit removes linked-expense behavior without changing existing accountless storage.

## Completion notes

Own checks and independent verification passed. Baseline-created permanent keys
for six accountless income/expense requests and linked income replayed identically
on the candidate without state/hash changes. Eight invalid-input differential
cases matched baseline. Fixed accountless expense hash pinned as a regression.
Full infrastructure suite not rerun: no schema changes; focused PostgreSQL cases
and workspace checks cover this unit. Native review and local commit pending.
