# Execution record: Report query profiling

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.10
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/103
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Profile the reporting queries with `EXPLAIN` and assess whether additional
indexes or query rewrites are justified. Provide a reproducible isolated probe,
not speculative production indexes or a machine-dependent performance gate.

## Existing state and evidence

Runtime baseline: `69d1150`. Preserve the uncommitted P09.06–P09.09 changes.
The current schema already provides owner/date transaction indexes, account/date
movement indexes, owner/state rule indexes, pending-occurrence owner/due-date
indexes and unique occurrence slots.

## In scope

Add `PostgresComposeTest.test_report_query_plans_against_disposable_postgres`.
It extracts the seven actual SQL literals from the Rust report handlers by
unique prefix, substitutes only fixed synthetic fixture parameters, and runs
`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)` inside repeatable-read, read-only
transactions with the same two-second lock/five-second statement timeouts.

The profile covers flow totals, category distribution, monthly flows, balance
history, planning rule retrieval, projection lookup and final planning totals.
It prints actual row counts, execution time, buffer hits/reads, temporary block
usage, scan types and selected index names. It checks result cardinalities,
required existing indexes, fixture sizes and unchanged financial table hashes.
It deliberately does not assert timings or exact planner choices.

## Out of scope

New schema indexes, migrations, query rewrites, HTTP behavior changes, production
data, remote databases and changes to the prior reporting regressions.

## Constraints and invariants

Use the existing disposable local Compose harness, synthetic users and automatic
container/volume/network cleanup. Do not disable sequential scans or force an
index plan. `EXPLAIN ANALYZE` executes the SELECT; read-only transactions and
before/after table hashes guard against unintended writes.

## Workload

| Dimension | Fixture |
| --- | --- |
| Owners | Small: 200 transactions / 2 accounts / 2 rules; wide: 20,000 transactions / 100 accounts / 100 rules |
| Transactions | 20,200 total; income/expense, posted/voided, SEK/EUR/USD, deterministic dates across five years |
| Movements | One signed movement per posted fixture transaction |
| Occurrences | 5,304 weekly slots, planned/skipped, across 102 rules |
| Date ranges | February 2024 and January 2020–December 2024 |
| History bound | 29 days or 366 days, not an unsupported five-year history request |
| Planning arrays | One pre-aggregated projection row and two candidate projection slots, including an already-persisted slot |
| Statistics | Explicit `ANALYZE` after fixture seeding |

Seven statements × two owners × two ranges = 28 plans. Rule and projection
lookups repeat across ranges because they do not consume the period filters.
This profiles database SQL components, not the Rust recurrence calendar loop.

## Steps

- [x] Map production query paths and existing indexes.
- [x] Add an isolated reproducible profile without copied SQL or forced plans.
- [x] Record the final workload results and index decision.
- [x] Run repository and whitespace checks; update only P09.10.

## Acceptance criteria

- [x] All 28 plans execute against migrated disposable PostgreSQL.
- [x] Expected row counts and unchanged fixture hashes are verified.
- [x] Record observed index selection and the reason for retaining/changing indexes.

## Verification

From the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_report_query_plans_against_disposable_postgres -v
python3 scripts/check_repository.py
git diff --check
```

| Check | Observed result |
| --- | --- |
| Initial profile | Passed; fixture subsequently spread small-owner dates across five years and added scan/temp-block diagnostics |
| Final 28-plan profile | Passed on PostgreSQL 18.6; 28 plans, cardinalities and unchanged hashes; harness 20.986 seconds, cleanup passed |
| Repository and whitespace checks | Passed after final changes |

## Measurements and index decision

Observed final wide-owner SQL timings and root shared-buffer hits:

| Component | February ms / hits | Broad-range ms / hits |
| --- | --- | --- |
| Flows | 0.775 / 107 | 8.910 / 378 |
| Category distribution | 0.970 / 111 | 6.621 / 382 |
| Monthly flows | 0.902 / 110 | 11.629 / 381 |
| Account history | 0.350 / 7 | 0.866 / 7 (366-day bound) |
| Rule retrieval | 0.368 / 6 | 0.288 / 6 |
| Projection lookup | 0.454 / 80 | 0.436 / 80 |
| Planning totals | 6.955 / 309 | 7.880 / 390 |

All 28 plans reported zero shared reads and zero temporary blocks. The selective
flow/distribution/monthly plans used `transactions_user_date_idx`; the small
owner's broad ranges used `transactions_user_id_idx`. History used
`account_movements_account_date_idx`. Selective planning used existing movement
and pending-occurrence indexes, and projection lookup used the unique slot index.
Broad ranges for the owner holding 20,000 of 20,200 transactions used sequential
scans; all-query row counts and read-only fixture hashes passed.

**Decision:** Retain existing indexes and SQL. This workload shows no timeout or
spill and no demonstrated need for another index. No candidate index or rewrite
was benchmarked, so no speedup or optimality claim is made. Adding a speculative
covering/partial index would impose storage, WAL and write-maintenance costs
without a measured benefit. Revisit with a larger or production-representative
workload if observed latency warrants it.

## Risks, assumptions, and decisions

These are warm, synthetic, single-client measurements, not production latency
or capacity guarantees. Parameter substitution permits constant-specific planning;
SQLx prepared/generic plans, cold buffers, write contention, full HTTP overhead,
maximum projection-slot arrays and much larger data sets are not benchmarked.
No index candidate is claimed faster without a before/after experiment.

Regression/profiling-only work has no expected behavioral RED. No production
code changes, so no differential runtime check is needed. The five HTTP suites
and Cargo formatting/Clippy/workspace checks passed earlier this session for
P09.09; they are not rerun for this Python profiling addition. RDD is globally off.

## Completion notes

Local verification complete; publication pending. Keep this record in `active/` until merge.
No commit, push or PR; preserve prior changes and `.atl/`.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `4d348d3 test(reports): verify reporting invariants and profile queries (P09.06-P09.10) (#103)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
