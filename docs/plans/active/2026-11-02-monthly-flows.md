# Execution plan: monthly realized flow history

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.04
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Approved `GET /v1/reports/monthly-flows?date_from=…&date_to=…`: monthly
income, expenses and net flow by currency, zero-filled months, clipped inclusive
edge periods, at most 120 calendar months. This is not balance history (P09.05).

## Existing state and evidence

P09.02 already defines verified-owner realized flow totals, exact decimal text and
read-only snapshot handling; P09.03 distribution reconciles with those totals.
Current uncommitted P09.01/P09.02/P09.03 work and .atl/ must remain intact.

## In scope

- Explicit inclusive dates, three currency series in SEK/EUR/USD order.
- Chronological month buckets with month key, effective dates and exact totals.
- Same accountless/interest/exclusion semantics as P09.02, current corrections only.
- Public HTTP tests, API/schema/product/setup docs and local verification record.

## Out of scope

Balance history, cumulative balances, forecasting, UI, extra filters/pagination,
migrations, dependencies, indexes, publication and other roadmap completion.

## Constraints and invariants

Never use floating point or narrow aggregate sums to i64. Reuse decimal-text
formatter and existing bearer/query limits. Parameterized owner predicate and
repeatable-read read-only transaction with bounded SQL timeouts. Calendar months
follow explicit business dates, not UTC audit timestamps or database session zone.
Risk: item 2 (authenticated financial ownership); item 3 (shared router/formatter).

## Steps

- [x] Observe public-interface absent-route RED.
- [x] Implement bounded calendar-series report and synchronized contract/docs.
- [x] Focused PostgreSQL GREEN, sibling regressions and workspace checks.
- [x] Independent ownership/read-only/correction/calendar verification.

## Acceptance criteria

Required strict ordered dates; inclusive boundary/clipped month periods; empty
months zero-filled; leap/year transitions and years 0001/9999; 120 months accepted,
121 rejected; exact totals beyond i64; currencies separate; each bucket reconciles
with flows; ownership, exclusions, accountless and actual interest; no writes.

## Verification

| Check | Observed result |
| --- | --- |
| Absent-route RED | Three HTTP groups failed because the absent report route lacked authenticated no-store responses. |
| Disposable `test_monthly_flows_against_disposable_postgres` | Initial 3 groups passed; final 4 passed after promoting independent probe (15.431s final harness). |
| Disposable `test_flow_reports_against_disposable_postgres` | Passed all existing flow groups. |
| Disposable `test_expense_distribution_against_disposable_postgres` | Passed all existing distribution groups. |
| `cargo fmt --all -- --check` | Passed after normalization. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed, including final promoted probe. |
| `cargo test --workspace --all-features --locked` | Passed; disposable-only database cases ignored there and separately exercised above. |
| `python3 tests/infrastructure/test_openapi.py -v` | All 6 structural guards passed; not full external OpenAPI validation. |
| `python3 scripts/check_repository.py`; `git diff --check` | Passed. |
| Independent fresh-scratch PostgreSQL probes | Four groups passed; no blockers. |

Independent public PATCH moves December expense into January with amount change;
public void then removes it. Every public table remains unchanged across accepted,
rejected and failing reads. Foreign sessions see only their own events; owner and
query injection reject. Planned/ungenerated recurrence slots do not become flows.
Dedicated pools confirmed Pacific/Kiritimati and America/Los_Angeles produce identical
JSON. An exclusive transactions-table lock yields safe `503 monthly_flows_unavailable`
with no partial currencies or table-name disclosure. Probe promoted into permanent tests.

No existing report parser/formatter logic changed; router merge only. Existing
reports' public regression suites passed. No CLI changed, so CLI history-differential
checks are not applicable. Prior uncommitted reports are not attributed to HEAD.

## Risks, assumptions, and decisions

120 counts intersected calendar months inclusively, not elapsed days or complete
months. Each month row contains `month` (YYYY-MM), effective `date_from`/`date_to`,
`income`, `expenses`, `net_flow`; outer rows contain `currency` and `months`.
Future dates are valid filters, not forecasts. No implicit profile-zone date.

## Completion notes

Implemented `monthly_flows.rs`, safe error and router wiring, exact decimal-text
reuse, API schemas/examples, product/setup docs and permanent database/schema guards.
Civil month generation explicitly uses timestamp without time zone; date formatting
is explicit. Validation counts calendar month indices before SQL, including supported
extremes, so the zero-filled series is bounded to 360 currency/month rows.

No new dependencies, migrations, indexes, UI or balance-history behavior. Query-plan
profiling remains P09.10; no large-data performance guarantee claimed. Prior
P09.01/P09.02/P09.03 and .atl/ preserved. Independent scratch removed after promotion
and parent focused rerun. RDD globally off; no review lifecycle, commit or remote operation.
Keep this record in active/ until a future PR merges. Next: P09.05.
Risk: items 2 and 3 (financial ownership and shared router/formatter), independently verified.
