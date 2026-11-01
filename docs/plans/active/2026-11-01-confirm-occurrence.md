# Execution plan: Confirm recurring occurrences

- **Status:** Verified locally — PR #93 open, merge pending
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P07.10
- **Tracking issue / PR:** [PR #93](https://github.com/cuenora/cuenora/pull/93); no issue by maintainer approval
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Confirm a planned recurring occurrence using actual payment terms, posting one
expense and linking it atomically. Follow the approved
[lifecycle contract](../../product/recurrence-lifecycle.md); skipping and
postponing remain P07.11.

## Existing state and evidence

- Main `c096e55` includes P07.09; occurrence generation does not post expenses.
- Migration 0017 protects occurrence ownership, scheduled identity, expense kind,
  one-to-one transaction linkage and terminal history.
- `expense_creation.rs` implements verified-owner expense posting, audit and
  permanent normalized retries, but currently owns its database transaction.
- RL08/RL09 specify accountless retry and rejected-account invariants.

## In scope

Confirmation API, composable existing expense posting, durable retry semantics,
isolated PostgreSQL regression tests, OpenAPI and documentation.

## Out of scope

Skipping/postponing, mobile applications, scheduler changes, deployment, merging,
and changes to already-posted expense correction semantics.

## Constraints and invariants

Reuse existing validation/posting rules. Confirmation, expense, optional movement
and audit must commit together. Matching retries must not repost, even after
later expense corrections. Reject conflicting intent and foreign ownership
without changing financial or occurrence data. Planned terms are not actual
payment terms. Preserve all existing expense retry hashes and API responses.

## Steps

- [x] Resolve delivery strategy before work-unit commits.
- [x] Observe RED for confirmation through the public HTTP interface.
- [x] Implement atomic confirmation and observe GREEN.
- [x] Run compatibility, workspace and isolated PostgreSQL checks.
- [x] Independently verify ownership, atomicity, retries and concurrency.
- [x] Update contracts and roadmap only with observed completion evidence.

## Acceptance criteria

- [x] Actual accountless/linked expense posts once; only linked posting moves money.
- [x] Matching retries return the same result; conflicting/new terminal actions fail.
- [x] Invalid owner/account/category/date requests preserve all affected data.
- [x] Concurrent confirmation and late database failures cannot leave partial writes.
- [x] Existing expense creation and recurrence behavior remain compatible.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| New disposable PostgreSQL confirmation target | RED then GREEN | Initial 0/7 RED; final retained 13/13 passed | Passed |
| Confirmation-key migration preservation/linkage | Pass | Focused group passed; full suite also passed | Passed |
| Full isolated infrastructure, including existing expense and recurrence targets | Pass | 73/73 groups passed in independent scratch | Passed |
| Cargo fmt, Clippy and locked workspace tests | Pass | Final format/Clippy passed; 109 passed, 0 failed, 168 intentionally ignored database tests | Passed |
| Repository checks and OpenAPI YAML/local references | Pass | Repository and diff checks passed; all local references resolve | Passed |
| Differential HTTP probes | Unchanged old behavior | Baseline/candidate 18/18 income/expense/transfer statuses and normalized bodies identical | Passed |
| Independent risk verification | Pass | S1/S2 PASS; six cancellation/lock/replay probes retained; parent reran all 13 database cases | Passed |

## Risks, assumptions, and decisions

Risk items 1/2/3/4: additive persisted retry identity, authenticated ownership, shared expense posting and concurrent
confirmation. Migration rollback, populated-history preservation and exact-linkage constraints passed.
Initial estimate: 700–1100 authored changed lines; final pre-commit candidate
was 1571 lines across 20 files, including retained concurrency regressions.
The maintainer approved one coherent size-exception delivery. The maintainer explicitly authorized current-session gh PR creation and SSH push
to origin, one size-exception PR without an issue; no merge is authorized. Native review is globally off (disabled/unmanaged).

## Completion notes

Implemented strict actual-payment confirmation, composable ordinary-expense posting,
additive permanent retry identity, OpenAPI/product/storage docs and ADR-040.
All applicable checks above passed. Initial Clippy found a needless borrow from
helper extraction; it was fixed without suppression and final checks passed.

Independent verification covered both public archive/confirmation queue orders,
post-wait owner/account revalidation, owner/audit-stage cancellation with full
public-table hashes unchanged, cross-owner keys and replay after public expense
edit/archive/profile-zone changes. Six probes were retained in the ordinary
confirmation target; the parent reran all 13 cases successfully. The full suite
ran before retaining those six probes, which separately passed in scratch and
again after retention. Production implementation bytes did not change.

One coherent size-exception work unit includes behavior, schema, tests and docs;
rollback scope is this confirmation unit plus its shared-helper extraction, using
forward schema changes rather than deleting history. PR #93 is open against main; merge remains pending.
P07.11 skip/postpone and P07.12 dedicated no-premature-accounting proof remain open.
Verification is bounded, not exhaustive concurrency model checking.
Recovery log: `odd/tasks/p07-10-confirm-occurrence.md`.

Implementation commit: `d4f1dfd8b63d31ec49554c655e8ccc062f8b486e` (1573 authored changed lines,20 files).
Local feature branch: `feat/p07-10-confirm-occurrence`. PR #93 is open against main; merge remains pending.

Publication uses exactly `type:feature` and `size:exception`; CI was queued at
creation and is not claimed passed. Source implementation and functional proof
are unchanged by this publication-only evidence update.
