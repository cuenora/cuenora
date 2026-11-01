# Execution plan: Skip or postpone recurring occurrences

- **Status:** Verified locally — approved size-exception delivery
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P07.11
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Skip a planned occurrence or move its effective due date forward without posting
money or changing its original slot and recurrence calendar.

## Existing state and evidence

Main `46c09a9` includes confirmation. Migration 0017 already separates scheduled
and due dates, preserves terminal history, and protects ownership. Generation
preserves existing slots. See [lifecycle](../../product/recurrence-lifecycle.md).

## In scope

Strict owner-scoped planning actions, permanent retries, isolated PostgreSQL tests,
OpenAPI and product/storage documentation. Recovery: `odd/tasks/p07-11-occurrence-actions.md`.

## Out of scope

Reopening skipped/confirmed items, changing planned terms or rule schedules,
mobile UI, P07.12 dedicated accounting verification, publication and merge.

## Constraints and invariants

Only planned occurrences accept new actions. Postponement must be strictly later
than current due_on, may exceed the rule end, and preserves state/identity/terms.
No financial events, movements or transaction audits. Existing paused/ended rules
remain actionable. Serialize owner first, occurrence second; commit retry and
mutation together. Preserve existing HTTP behavior.

## Steps

- [x] Resolve postponement date policy with the maintainer.
- [x] Observe public-interface RED before implementation.
- [x] Implement and observe GREEN, including retry and rollback tests.
- [x] Run workspace, full isolated infrastructure and compatibility checks.
- [x] Independently verify risk items 1/2/4; parent reruns focused checks.
- [x] Resolve delivery strategy before committing; record verified outcomes.

## Acceptance criteria

- [x] Skip and postpone preserve schedule/terms and never post money.
- [x] Only strictly later dates are accepted, including beyond rule end.
- [x] Authentication/ownership, terminal states and invalid input reject atomically.
- [x] Permanent retries, concurrent actions and late failures preserve consistency.
- [x] Generation and confirmation retain their prior behavior.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| New disposable PostgreSQL action target | RED then GREEN | Initial route RED, object-only RED, final 12 action groups passed | Passed |
| Populated-history migration/constraints | Pass | Focused history/rollback/immutability group passed | Passed |
| Full isolated infrastructure suite | Pass | 74/75 passed in 1658.088s; only failure was omitted new table in expected catalog. Fixed; scoped independent recheck passed in 31.716s | Passed after correction |
| Cargo fmt, Clippy, locked workspace tests | Pass | Final fmt/Clippy passed; workspace 110 passed, 0 failed, 180 intentionally ignored DB cases | Passed |
| Repository/OpenAPI checks | Pass | Repository validation, YAML/local references and diff whitespace passed | Passed |
| Baseline/candidate existing HTTP differential | Identical | 12/12 normalized status/body outcomes matched; both subprocesses exit 0 | Passed |
| Independent risk probes and parent spot check | Pass | S1/S2 PASS; four independent groups passed; parent reran all 12 retained action cases + migration, 2/2 harness groups in 60.196s | Passed |

## Risks, assumptions, and decisions

Risk: items 1/2/4 (additive persisted retry identity, authenticated ownership,
concurrent state changes). Reuse owner-first recurrence authorization and key
validation. Owner-wide action namespace shared by skip/postpone, separate from
confirmation/rules. Original results replay even after subsequent actions.
No profile-zone requirement: these actions derive no financial date from today.
Initial forecast 900–1300 authored lines; final candidate approximately 1900,
including strict-object validation and retained deterministic concurrency/cancellation
regressions. One slicing pass found a coupled action engine/retry ledger with
shared invariants, tests and contract; separating file types would leave incomplete
behavior. The maintainer explicitly approved one coherent `size:exception` delivery
on 2026-11-01; no push, PR publication or merge is authorized.
Native review is globally off; do not enable it. No remote authorization.

## Completion notes

Implemented planning-only actions and permanent retry storage, OpenAPI/product/storage
docs and ADR-041. Twelve public HTTP groups and populated-history migration checks
passed in the parent's final run. Final fmt, Clippy, workspace and repository/contract
checks passed. Independent S1/S2 verdict is PASS with no open blockers.

The full isolated suite ran once on the frozen candidate: 74/75 passed. The sole
candidate-caused failure was a test-catalog omission, not a runtime failure; baseline
passed that case. One correction added the new table to the expected schema catalog.
Independent scoped recheck passed; no second full sweep was run. The verifier
confirmed all 65 production/build/migration files still matched frozen bytes/modes.
Two independently authored ordered-action and competing-date probes were retained
in the action target and passed again in the parent's 12-case run.

Compatibility covered valid rule create/edit/pause/resume, confirmation/replay,
new-field rejection and terminal confirmation with its unchanged original message.
Four independent probe groups covered ordered skip/postpone queues, original replay,
revoked/foreign ownership, post-update cancellation, and competing forward dates.

Initial Clippy unused-must-use warning was fixed without suppression. Derived
structs accepted positional arrays; a retained RED case justified map-only
deserialization without weakening duplicate-field checks.

Local work-unit commit is authorized under the approved size exception. No
publication, remote access or merge. Proof is local and bounded, not exhaustive
concurrency model checking.
P07.12 and mobile workflows remain separate. Rollback boundary is this additive
planning-action unit and visibility-only helper exposure; deployed retry history
must be preserved through forward migrations, never destructive rollback.
