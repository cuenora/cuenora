# Execution plan: Idempotent due-occurrence generation

- **Status:** Verified; merge pending
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P07.08
- **Tracking issue / PR:** One maintainer-approved size-exception PR from `feat/p07-08-due-occurrences`; issue waived
- **Owner:** Agent with maintainer decisions

## Goal and user-visible outcome

Generate each eligible due expense occurrence once, as planning data only.
See [specification and log](../../../odd/tasks/p07-08-due-occurrences.md).

## Existing state and evidence

P07.07 provides serialized verified-owner lifecycle operations; migration 0017
provides unique scheduled slots, immutable identity, pause exclusions and an
inclusive progress cursor. The pure next_occurrence calculator is implemented.

## In scope

Internal application generation through rule-local today inclusive, active backlog,
pause/edit coverage, inclusive end, immutable snapshots and atomic retry behavior.

## Out of scope

P07.09 worker, scheduling, HTTP routes, occurrence actions and financial posting.
No migrations, dependencies, remote operations or real data.

## Constraints and invariants

Reuse owner → rule locks and exact-money/calendar helpers. Generate only planned
rows and monotonic progress atomically; preserve existing occurrences and financial
rows. Capture the trusted server clock after acquiring locks.

## Steps

- [x] Observe RED for deterministic application-level PostgreSQL regressions.
- [x] Implement and document generation, then observe GREEN.
- [x] Run repository/workspace checks and independent risk verification.

## Acceptance criteria

- [x] Rule-local today, inclusive end, anchored calendars and active backlog.
- [x] Paused rules and excluded/covered dates are not generated.
- [x] Retries and concurrent runs preserve one immutable snapshot per slot.
- [x] Failed runs preserve both occurrences and progress; no financial side effects.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Disposable recurring_generation target | Generation acceptance passes | Original 6 RED → GREEN; final 10/10 passed | Passed |
| Rust fmt, Clippy, workspace tests | Pass | Passed; final Cargo 106 passed, 145 DB cases intentionally ignored | Passed |
| Repository checks | Pass | Repository validation and diff whitespace passed | Passed |
| Independent risk verification | No change-caused blockers | S1–S4 passed; lock/edit/pause/cancellation probes, 80 identical baseline/candidate HTTP responses | Passed |

## Risks, assumptions, and decisions

Risk items 2/4: server-enforced ownership and serialized persisted writes.
Maintainer confirmed due-only generation through rule-local today, recovering
active backlog. RDD remains off. Maintainer approved one size-exception PR without an issue.

## Completion notes

Implemented the callable generator and retained independent regression probes.
The full isolated PostgreSQL infrastructure suite passed 70/70 before retaining
three additional verifier probes; the final focused target passed 10/10 afterward.
Production bytes were unchanged by retaining those probes. Final fmt, Clippy,
workspace tests and the parent clock-lock spot-check passed. Initial Clippy caught
item ordering; moving the function before the test module resolved it without
warning suppression. No checks remain failed or skipped except intentional DB
ignores in ordinary Cargo (executed explicitly through the harness).

Independent edit/pause probes use equivalent locked database transitions rather
than overlapping HTTP requests. Cancellation was tested while lock-blocked;
late-insert failure covers write rollback. RDD remains globally disabled.

One coherent generation unit exceeds the approximate 400-line budget; splitting
runtime from its tests/contract would break the work-unit boundary. Maintainer
approved a single size-exception PR using the current gh session and SSH push,
without an issue. Implementation commit: `d9e6d3200fefcd80c15a25609d6a31c34c934e5e`.
No merge is authorized.
P07.09 worker and later occurrence actions remain separate.
