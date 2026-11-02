# Execution plan: expense distribution by category

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.03
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Approved `GET /v1/reports/expense-distribution?date_from=…&date_to=…`:
exact expense totals per category/currency, including uncategorized, and two-place
half-even percentages. Rounded shares need not sum to 100%; zero spending has no
percentages. Reuse P09.01/P09.02 realized expense and inclusive date semantics.

## Existing state and evidence

P09.02 provides verified-owner read-only flow totals and exact decimal formatting.
The immutable built-in catalog has seven expense categories. Category is nullable.
Existing P09.01/P09.02 work and .atl/ must be preserved; no commits authorized.

## In scope

- Fixed SEK/EUR/USD rows; seven catalog categories plus explicit null-category bucket.
- Posted expenses only, accountless included, current corrected events once.
- Public HTTP regressions, currency reconciliation, exact tie examples and safe errors.
- OpenAPI, product/setup documentation and this local execution record.

## Out of scope

UI, category mutations, filters/pagination, monthly/history/pending reports,
migrations, dependencies, indexes, publication and other roadmap completion.

## Constraints and invariants

No floating-point money/percentages or i64 narrowing of aggregate sums. Reuse the
existing exact decimal formatter, strict date type and bearer middleware. Parameterized
owner predicate, repeatable-read read-only snapshot and bounded SQL timeouts.
Risk: item 2 (financial ownership); item 3 (shared formatter visibility and report routing).

## Steps

- [x] Observe RED for the absent endpoint.
- [x] Implement isolated report and synchronize API documentation.
- [x] Focused PostgreSQL GREEN, sibling regressions, workspace checks.
- [x] Independent ownership/rounding/read-only verification.

## Acceptance criteria

Inclusive dates, exact currency/category sums reconcile with flows; uncategorized spending
represented by null identity (not Other expense); ties round half-even; empty totals
have null percentages; no user data leaks or writes; strict transport validation.

## Verification

| Check | Observed result |
| --- | --- |
| Absent-route RED | Three public HTTP groups failed because the absent route did not provide authenticated no-store report responses. |
| Disposable `test_expense_distribution_against_disposable_postgres` | Three initial groups passed; four final groups passed after promoting independent regression (12.846s final harness). |
| Disposable `test_flow_reports_against_disposable_postgres` | Passed all existing flow groups. |
| Disposable `test_categories_against_disposable_postgres` | Passed existing catalog HTTP tests. |
| `cargo fmt --all -- --check` | Passed after normalization. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed, including final promoted test. |
| `cargo test --workspace --all-features --locked` | Passed; database tests ignored there and exercised separately. |
| `python3 tests/infrastructure/test_openapi.py -v` | Five structural guards passed; not full external OpenAPI validation. |
| `python3 scripts/check_repository.py`; `git diff --check` | Passed. |
| Independent fresh-scratch disposable PostgreSQL probes | Four groups passed; no candidate-caused blockers. |

Independent coverage includes public PATCH category/amount/date changes and void;
archived-account expense retention; income/interest/opening/transfer/planned-payment
exclusion; owner/filter injection isolation; hashes of every public table unchanged;
flow reconciliation and an independent u128 rational oracle at an 8e18 denominator,
including even/odd half ties and single-minor-unit neighbors.

Only shared formatter visibility changed, not its logic. Existing flow output and
catalog behavior were exercised through their public HTTP regression suites. No
CLI command changed; CLI history-differential probes are not applicable.

## Risks, assumptions, and decisions

Stable order: SEK/EUR/USD; ascending catalog UUID and uncategorized last. Zero
category rows retained; percentage is null iff currency total is zero. Percentages
use that currency's full expense denominator, including uncategorized amounts.

## Completion notes

Implemented the isolated report module with parameterized owner-scoped SQL,
exact numeric div/mod half-even rounding and the existing decimal-text formatter.
Added safe error/router wiring, OpenAPI/schema guard, public regression tests,
product/setup documentation and local master-plan completion. Promoted the independent
probe into the fourth permanent regression group and reran it after formatting.

No new dependencies, migrations, catalog writes, indexes, CLI or UI. Database-outage
injection and performance profiling were not run; safe failure mappings inspected,
profiling remains P09.10. Prior P09.01/P09.02 work and .atl/ preserved.
RDD globally off; no remote operation, review lifecycle, commit or publication.
Keep this record in active/ until merged. Next task: P09.04.
Risk: items 2 and 3 (financial ownership and shared formatter/router), independently verified.
