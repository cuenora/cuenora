# Execution plan: Recurrence storage

- **Status:** Active
- **Date:** 2026-10-31
- **Roadmap task ID(s):** P07.03
- **Tracking issue / PR:** Not created

## Goal and scope

Add `recurring_rules` and `recurring_occurrences` without changing historical
financial records. Follow the [lifecycle](../../product/recurrence-lifecycle.md)
and [calendar](../../product/recurrence-calendar.md) specifications. No recurring
API, worker, calendar calculator or automatic posting is part of this slice.

## Constraints and verification

Preserve exact money, owner/currency/category integrity and planned-versus-realized
separation. Test fresh and populated migration, invalid writes, stable scheduled
identity, reference changes and rollback against isolated PostgreSQL. Risk items
1 (persistent schema) and 2 (ownership) require independent verification.

## Progress and evidence

See the [task log](../../../odd/tasks/p07-03-recurrence-storage.md) for specs,
RED/GREEN evidence, commands, review mode and remaining work. Implementation and
verification are complete locally; the maintainer approved a single size-exception PR. Commits are being prepared.
Native review is globally disabled; do not enable it.

## Observed verification

- RED: three new database groups failed before migration (absent tables/file).
- GREEN: three recurrence groups and fresh-schema reproducibility passed (4 tests).
- Full isolated infrastructure suite: 67 tests passed in 1345.710 seconds.
- Cargo format, Clippy with warnings denied and locked workspace tests passed;
  Cargo reported 93 passed and 126 explicitly ignored database tests.
- Independent S1–S3 verification: two scratch-copy probes passed; 45 rejected
  writes preserved all public-table hashes; populated/failed-DDL rollback passed.
- Added the verifier's unique-expense-link probe to regression coverage; the
  reference/confirmation group passed again (20.622 seconds).
- Parent format spot check, repository validation and diff whitespace check passed.
- Native review: disabled globally, unchanged. No remote operation or commit.

## Approved delivery

P07.03 behavior and checks are complete. The maintainer explicitly approved one
size-exception PR including the prerequisite P07.01/P07.02 specifications and
P06.08/P06.09 verification notes. Push via the existing SSH origin and create the
PR using the current gh session; do not merge. Native review remains disabled.
Preserve `.atl/`. Recurrence APIs/calculation/generation remain P07.04–P07.12.

Prerequisite commits: `7d9b078`, `5468310`, `83f624d`. The schema, its regression
tests and storage documentation remain one dedicated work unit in the same PR.
