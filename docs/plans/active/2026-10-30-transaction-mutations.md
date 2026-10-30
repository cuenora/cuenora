# Execution plan: Atomic transaction corrections

- **Status:** Active
- **Date:** 2026-10-30
- **Roadmap task ID(s):** P05.13
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Edit and void all five recorded event kinds without overwriting movements or duplicating balance effects. The authoritative task/spec/evidence log is [P05.13](../../../odd/tasks/p05-13-transaction-mutations.md).

## Existing state and evidence
Income/expense creation and opening creation post atomically. Recorded history reads all five kinds. Event rows are mutable; movements/audit and original creation keys are immutable. There are no item mutation endpoints.

## In scope
Owner-verified edits/voids, permanent mutation retries, linked compensation, minimal audit, real isolated PostgreSQL and public HTTP tests.

## Out of scope
Transfer/interest creation, forecasts, restoration, kind conversion, rate calculation, mobile clients, merging or releasing.

## Constraints and invariants
Exact money, owner isolation, normal-account-only ordinary income, no accountless movements, same-currency transfer equality, atomic two-leg correction. Archived affected accounts must be reactivated for movement changes; metadata-only edits remain allowed.

## Steps
- [x] Define correction contract and observe public-interface RED.
- [x] Implement atomic correction/retry protocol and verify compatibility.
- [ ] Independent verification passed; native review and commit pending.

## Acceptance criteria
S1–S3 in the authoritative task file cover behavior, all five kinds and archived account rules. Rejected requests leave financial/audit/retry state unchanged. Original postings and creation retries remain intact.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused disposable PostgreSQL mutation target | All acceptance tests pass | All ten expanded DB groups passed | Passed |
| Pinned Cargo format, Clippy, workspace tests | Pass; DB cases explicitly exercised separately | Format/Clippy pass; 91 passed/105 ignored | Passed |
| Full disposable PostgreSQL suite and repository checks | Pass | 61 tests pass; repository checks pass | Passed |
| Independent isolated baseline/candidate verification | No change-caused blockers | S1–S3 pass; 16 HTTP observations identical; archive race spot check passed | Passed |

## Risks, assumptions, and decisions
Risk items 1 (persisted financial corrections), 2 (ownership gates), 4 (concurrent retries/archive). All five kinds and archive policy explicitly approved. See the task log for engineering contract and delivery-size decision.

## Completion notes
Implemented locally; pinned format/Clippy and Cargo passed (91 passed, 105 ignored), full isolated PostgreSQL suite passed 61 cases. Independent S1–S3 verification and 16-observation baseline HTTP compatibility passed; parent archive-race spot check passed. Independent regressions added to the ten-group focused target. Maintainer approved the single-PR size exception and publication via current gh session over HTTPS. Commit and native candidate review pending; no merge authorized.
