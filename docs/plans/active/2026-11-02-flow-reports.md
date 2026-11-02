# Execution plan: realized income and expense aggregates

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.02
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Implement the approved `GET /v1/reports/flows?date_from=2026-11-01&date_to=2026-11-30`
contract: inclusive income, expenses and net flow per currency, decimal strings.

## Existing state and evidence

Transactions already store exact minor units, posted/voided states and credited
interest. Existing listing uses verified-owner repeatable-read read-only queries.
P09.01 defines realized metrics; reporting endpoints do not yet exist.

## In scope

- Explicit required dates and fixed SEK/EUR/USD rows, including exact zeros.
- Posted income/interest and expenses, accountless included; openings/transfers/voids excluded.
- Public HTTP tests, safe errors, OpenAPI and product/setup documentation.

## Out of scope

Category/monthly/history/pending projections, new filters, migrations, indexes,
dependencies, publication, P06.10 completion and other Phase 09 tasks.

## Constraints and invariants

Reuse bearer identity and BusinessDate validation; isolate owners server-side.
Sum PostgreSQL numeric without narrowing totals to i64 or using floating point.
No data writes. Existing P09.01 changes and untracked .atl/ are preserved.
Risk: item 2 (authenticated financial ownership). Independent verification required.

## Steps

- [x] Observe public-interface RED, implement endpoint and exact aggregates.
- [x] Synchronize contract/docs and observe focused GREEN and workspace checks.
- [x] Independent ownership/correctness verification; record actual results.

## Acceptance criteria

Both boundaries included; all three currencies distinct; empty totals zero;
accountless and actual interest included; voids/openings/transfers excluded;
foreign data excluded; invalid/unknown/duplicate filters reject; no writes.

## Verification

| Check | Observed result |
| --- | --- |
| Baseline new public HTTP tests | RED: endpoint absent; both groups failed because 404 lacked authenticated no-store response. |
| Disposable `test_flow_reports_against_disposable_postgres` | GREEN: initial 2 groups, then final 3 groups including promoted independent regression; final harness 12.514s. |
| Disposable `test_transaction_listing_against_disposable_postgres` | Passed all 5 existing listing groups as sibling route regression. |
| `cargo fmt --all -- --check` | Passed after final normalization. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed; repeated after promoting regression. |
| `cargo test --workspace --all-features --locked` | Passed; ordinary PostgreSQL cases ignored, separately exercised above. |
| `python3 tests/infrastructure/test_openapi.py -v` | Passed all 4 structural guards; not full external OpenAPI validation. |
| `python3 scripts/check_repository.py`; `git diff --check` | Passed. |
| Independent fresh-scratch PostgreSQL probes | Passed: public correction/date move/void, inclusive same-day, foreign owner/session/query injection, future ungenerated recurrence and every-public-table no-write snapshots. |

Existing route behavior is unchanged: new routing merge only, no existing filter,
parser or handler mutation. Transaction listing regression passed. No CLI behavior
changed, so CLI baseline differential commands are not applicable.

## Risks, assumptions, and decisions

Approved endpoint and semantics in chat. Fixed currency rows avoid ambiguous absent
versus zero values; no currency selector or pagination is introduced.

## Completion notes

Implemented `flow_reports.rs`, strict HTTP query handling and verified-owner SQL
aggregation; router and safe-error wiring; contract/product/setup documentation;
three permanent database regression groups and structural OpenAPI guard. Aggregate
sums retain PostgreSQL numeric precision through decimal-text formatting, including
totals beyond i64. All currency rows have deterministic zero values.

No index/migration/dependency added. Query plan profiling and larger reporting
filters remain P09.09/P09.10; no performance guarantee is claimed. Database-outage
injection was not run; safe failure mappings were inspected. No mobile UI added.
Independent probes were promoted into permanent tests. Parent readback and focused re-run passed. Scratch cleanup was denied by the
runtime; retained outside the repository: `/tmp/cuenora-p0902-verify.absoMk`,
`/tmp/cuenora-p0902-verify.absoMk-cargo`, and `/tmp/cuenora-p0902-checks.Vyw6z8/`. Preexisting P09.01 changes and .atl/ preserved.
RDD globally off; no review lifecycle, commit, remote publication or merge.
Keep this execution record in active/ until a future PR merges. Next: P09.03.
Risk: item 2 (authenticated financial ownership), independently verified.
