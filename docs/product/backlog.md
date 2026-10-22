# Initial product backlog

**Version:** 0.1 · **Date:** 2026-10-22 · **Roadmap:** P00.10

This backlog connects [PRD v0.1](prd.md) to the existing roadmap. All items below
are planned implementation work, not completed features or created GitHub issues.
The [master plan](../plans/master-plan.md) remains the task-ID and completion
record. Value order follows the [prioritized stories](user-stories.md); shared
foundations must be delivered before the behavior depending on them.

## Shared prerequisites

| Area | PRD | Roadmap | Completion evidence required |
| --- | --- | --- | --- |
| Exact money, persistence, ownership constraints, audit history | R-01–R-04 | Phase 03 (P03.01–P03.12) | Money tests, isolated migrations, constraint and cross-user rejection tests. |
| Authentication and access isolation | R-01 | Phase 04 | Session and authorization tests, including FIN-012. |
| Shared mobile logic, API compatibility, native platform foundations | R-10 | Phase 10; P11.01–P11.03; P12.01–P12.04 | Contract fixtures, platform builds/tests, and first-account flows. |

## Value-ordered behavior

| Value order | Story | PRD | Implementation tasks | Acceptance examples |
| --- | --- | --- | --- | --- |
| 1 | US-01: recorded account balances together | R-02, R-07 | P05.01–P05.06; P09.05–P09.06; P11.04; P12.05 | EX-01; FIN-001, FIN-013 |
| 2 | US-02: upcoming payments by source account | R-05 | P07.01–P07.09; P11.08; P12.09 | EX-08; FIN-008, FIN-014 |
| 3 | US-03: possible per-account funding shortfalls | R-06 | P09.01 specifies boundaries; P09.05 implements pending totals and planning differences; P11.10/P12.11 present them using P07 occurrences and posted balances | EX-08; FIN-020 |
| 4 | US-04: actual transfers between accounts | R-04 | P06.01–P06.10; P11.07; P12.08 | EX-05–EX-07; FIN-005–FIN-007, FIN-017–FIN-018 |
| 5 | US-05: dated income and expenses | R-03, R-08 | P05.07–P05.14; P11.05–P11.06; P12.06–P12.07 | EX-02, EX-04, EX-10; FIN-004, FIN-015 |
| 6 | US-06: confirm recurring payments | R-05 | P07.10–P07.12; P11.08; P12.09 | EX-08; FIN-009 |
| 7 | US-07: spending by category and period | R-07 | P09.01–P09.04; P09.06–P09.10; P11.10; P12.11 | EX-02–EX-04; FIN-002–FIN-003, FIN-013 |
| 8 | US-08: distinguish balances, net flow, commitments, and savings | R-06–R-08 | P09.01–P09.07; P11.10; P12.11 | EX-01, EX-05, EX-08, EX-10 |
| 9 | US-09: record without an account and assign later | R-03 | P05.08–P05.14; P09.08; P11.05; P12.06 | EX-03–EX-04, EX-10; FIN-002–FIN-003 |
| 10 | US-10: interest forecasts versus actual payouts | R-08–R-09 | P08.01–P08.12; P11.09; P12.10 | EX-09; FIN-010–FIN-011, FIN-021–FIN-022 |

Value order is not permission to skip accountless income, savings restrictions,
or other invariants when implementing a higher-ranked story. No additional
top-level roadmap IDs are introduced by this index.

Implementation dependency order is recorded in the master plan's
[current implementation order](../plans/master-plan.md#13-current-implementation-order).
In particular, P09.01/P09.05 can follow account balances and recurring schedules
without waiting for interest forecasts. P15.06 is a retained, superseded planning
slot: rate changes are MVP P08.01/P08.08 work, not a deferred feature.

## Verification and release work

Phase 13 provides shared E2E, regression, authorization, and failure coverage;
Phase 14 provides operational/security readiness including backups and restore.
Android/iOS platform checks remain required alongside shared tests. The
[worked examples](worked-examples.md) and broader
[release cases](../plans/master-plan.md#8-acceptance-examples-required-before-public-release)
are specifications to turn into automated checks, not passing test results.

## Deferred work

The PRD's release boundaries and the master plan's release table govern later
work: v1 budgets, goals, reminders, export, and localization; later offline/import,
banking, reconciliation, shared-finance, and automatic-FX capabilities. No deferred
item becomes MVP work merely because it appears in the long-term roadmap.

Before selecting an implementation task, resolve the applicable PRD specification
prerequisites and use one focused roadmap unit with its tests and documentation.
