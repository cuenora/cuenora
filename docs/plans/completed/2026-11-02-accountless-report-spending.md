# Execution record: Accountless report spending

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.08
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/103
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Verify “Include accountless spending in financial reports.” Existing reports
include posted expenses with no account, in their currency and business-date
period. This task verifies existing behavior rather than adding duplicate logic.

## Existing state and evidence

Runtime baseline: `69d1150`. Preserve P09.06/P09.07 documentation changes and
the pre-existing untracked `.atl/` directory.

All three report queries aggregate `transactions` directly, without an account
join or an `account_id IS NOT NULL` filter:

| Report and source | Existing public HTTP acceptance evidence |
| --- | --- |
| Flows: `backend/crates/api/src/flow_reports.rs` | `exact_totals_include_boundaries_accountless_and_interest_but_not_other_events` in `tests/flow_reports.rs` seeds accountless SEK 3.05 and EUR 7.99 expenses; the exact response includes both and their net-flow effects. |
| Categories: `backend/crates/api/src/expense_distribution.rs` | `categories_reconcile_by_currency_with_inclusive_dates_and_no_writes` in `tests/expense_distribution.rs` uses an expense helper that omits account ID. Categorized and uncategorized SEK expenses total 0.03; accountless EUR expenses total 184467440737095516.14. Category amounts/percentages reconcile with the matching flow report. |
| Monthly: `backend/crates/api/src/monthly_flows.rs` | `monthly_history_zero_fills_clips_edges_and_reconciles_exact_currency_flows` in `tests/monthly_flows.rs` includes accountless EUR 7.99 in February and USD 0.25 in the clipped March period, excludes the following day's expense and reconciles every monthly currency bucket with flows. |

Test paths above are relative to `backend/crates/api/`. Existing fixtures also
exercise owner/date/state boundaries. Distribution and monthly tests compare
all-public-table snapshots before and after report reads, so these reads cannot
create account movements or alter stored balances.

## In scope

Run the three existing report suites against disposable local PostgreSQL, record
observed results and update only P09.08 after successful verification.

## Out of scope

New creation/reassignment flows, changes to account balances, implicit cash
accounts, migrations, new endpoints and other roadmap tasks. This verification
does not rerun expense-creation or assignment acceptance suites.

## Constraints and invariants

Follow `docs/product/accountless-entries.md`: report each expense once in its
currency and business-date period; do not silently allocate it to an account.
Keep categorized versus uncategorized distinct from assigned versus accountless.
Use synthetic data and disposable local PostgreSQL only.

## Steps

- [x] Inspect report queries and accountless public HTTP fixtures/assertions.
- [x] Execute three relevant report suites.
- [x] Record evidence and mark P09.08 locally verified.

## Acceptance criteria

- [x] Period spending and net flow include accountless posted expenses.
- [x] Category distribution includes categorized and uncategorized accountless spending.
- [x] Monthly series include accountless spending in the correct date/currency bucket.
- [x] Report reads preserve stored financial data.

## Verification

From the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_flow_reports_against_disposable_postgres \
  PostgresComposeTest.test_expense_distribution_against_disposable_postgres \
  PostgresComposeTest.test_monthly_flows_against_disposable_postgres -v
python3 scripts/check_repository.py
git diff --check
```

| Check | Observed result |
| --- | --- |
| Disposable PostgreSQL report targets | Passed: 3 harness targets in 36.437 seconds; 3 flow + 4 distribution + 4 monthly Rust HTTP tests (11 total) |
| Repository validation and whitespace | Passed after documentation changes |

## Risks, assumptions, and decisions

No runtime or test edits are needed if existing assertions pass. RED→GREEN does
not apply to a verification-only task. Full workspace Cargo tests, Clippy and
formatting are not rerun for passive documentation; database-backed report
tests execute explicitly rather than remaining ignored.

## Completion notes

Local verification only; publication pending. Keep this record in `active/`
until merge. No commit, push or PR; preserve preceding uncommitted work.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `4d348d3 test(reports): verify reporting invariants and profile queries (P09.06-P09.10) (#103)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
