# Execution plan: Account movement storage

- **Status:** Active
- **Date:** 2026-10-25
- **Roadmap task ID(s):** P03.08
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Create append-only account posting storage, not a posting API or balance service.
The authoritative task/spec/log is [the feature document](../../../odd/tasks/p03-08-account-movements.md).

## Existing state and evidence

Accounts and transactions expose composite owner-reference targets. The existing
PostgreSQL test harness uses disposable Compose databases and snapshot checks.

## In scope

- Movement migration, exact signed minor units, owned foreign keys and query indexes.
- Append-only guards, compensation storage tests and storage documentation/ADR.

## Out of scope

- P03.09 audit storage, P03.10 cross-table financial guards, posting/idempotency APIs.
- P05.13 editing/voiding protocol; no existing stored rows are changed.

## Constraints and invariants

Preserve original postings. Enforce owner-scoped references and exact amounts.
Use only synthetic records in isolated PostgreSQL. Preserve unrelated `.atl/`.

## Steps

- [x] Implement T1 / S1–S2 with observed RED then GREEN.
- [x] Run checks and independent verification; record evidence in the feature log.

## Acceptance criteria

- [x] Fresh migrations, owned references, bounds and indexes verified.
- [x] Original rows survive rejected mutations and compensation inserts.
- [x] Failed/reapplied migration is atomic; documentation states deferred guards.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused movement tests | RED before migration, GREEN after | See feature log | Passed |
| Full PostgreSQL suite | Pass in disposable Compose projects | See feature log | Passed |
| Repository check; Cargo fmt/clippy/test | Pass | See feature log | Passed |
| Independent verifier | No change-caused blockers | See feature log | Passed |
| Native four-lens review | No severe findings; acknowledged closure | Zero findings; authority burned | Passed |

## Risks, assumptions, and decisions

Risk: items 1 and 2 (financial migration and ownership). Append-only policy approved.
Normal application writes cannot update/delete/truncate; privileged DDL is not
prevented. Compensation orchestration is not implemented by this storage change.

## Completion notes

Implementation and independent verification passed: 25 PostgreSQL tests, 21 Rust
tests, format/Clippy/repository checks and two independent probes. Parent rerun:
two probes passed. Four native lenses returned zero findings; exact acknowledgement
closed the review of the candidate through `ab073c3`. Source remains unchanged.
No publication authorized. Keep this plan active until its PR is merged.
