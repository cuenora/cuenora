# Execution plan: Explainable savings forecast endpoint

- **Status:** Active
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P08.09
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Read an exact savings-interest forecast with explicit assumptions and payout-period breakdown, without posting money.

## Existing state and evidence

Pure daily/payout calculators and immutable configuration APIs exist. Before this task, persisted-history forecast orchestration did not exist. See [feature specification](../../../odd/tasks/p08-09-savings-forecast.md) for authoritative S1-S3, assumptions, tasks and verification log.

## In scope

Verified-owner future forecast, explicit bounded dates, flat snapshot balance, inclusive rates, fixed terms, exact breakdown, HTTP/OpenAPI documentation and isolated PostgreSQL regressions.

## Out of scope

Actual interest posting, historical replay, predicted transfers, receipt reconciliation and payout/capitalization transitions inside a window.

## Constraints and invariants

Reuse the pure calculator and existing identity/zone gates. Read balance and configuration in one consistent, read-only snapshot. Missing coverage and unsupported transitions fail explicitly. Preserve existing P08.07/P08.08 changes.

## Steps

- [x] T1: Public HTTP acceptance tests with observed RED.
- [x] T2: Implementation, contract and documentation with GREEN.
- [x] T3: Workspace and isolated PostgreSQL checks plus independent security/snapshot verification.

## Acceptance criteria

- [x] S1-S3 verified through HTTP with exact amounts and unchanged stored data.
- [x] Ownership, verification, profile zone, missing coverage and transition failures covered.
- [x] OpenAPI and product documentation match implemented fields and limits.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Disposable savings forecast HTTP harness | All groups pass; no persisted effects | Eight forecast groups + three existing configuration groups passed; final forecast run repeated after adopting independent regression | Passed |
| Cargo format, Clippy, workspace tests | Pass; ignored PostgreSQL distinguished | Formatting/Clippy passed; workspace 131 passed, 0 failed, 192 ignored | Passed |
| OpenAPI/repository checks | Pass | Three OpenAPI guards and repository validation passed | Passed |
| Independent risk items 2/4 probes | Ownership and consistent read-only behavior | Independent isolated S1–S3 probes passed; no blockers | Passed |

## Risks, assumptions, and decisions

Flat future balances and explicit rejection of payout/capitalization changes are user-approved. Future dates require confirmed profile-zone today. The 366-day window is an operational bound, not a bank term. First partial-period accrual excludes earlier days explicitly. RDD globally off; no remote authorization.

## Completion notes

Implementation and checks are complete locally. Commit/delivery strategy awaits a size-exception decision: the coherent endpoint with security/concurrency tests and contracts exceeds the approximately 400-line review heuristic. No commits or publication yet. P08.07/P08.08 changes remain uncommitted and preserved separately. Full task evidence is maintained in the linked feature specification.
