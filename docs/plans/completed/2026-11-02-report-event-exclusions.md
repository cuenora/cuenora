# Execution record: Realized report event exclusions

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.07
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/103
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Verify “Exclude transfers, account openings, and voided entries from normal
income/spending.” Existing report filters implement this requirement; no new
financial behavior is needed.

## Existing state and evidence

Runtime baseline: `69d1150`. Preserve the preceding P09.06 documentation changes
and pre-existing untracked `.atl/` directory.

| Boundary | Existing evidence |
| --- | --- |
| Period flows | `backend/crates/api/src/flow_reports.rs` selects only posted income, actual interest and expenses. `exact_totals_include_boundaries_accountless_and_interest_but_not_other_events` seeds posted transfers/openings and a voided expense inside the period, then asserts the exact full response. |
| Category spending | `backend/crates/api/src/expense_distribution.rs` selects only posted expenses. `categories_reconcile_by_currency_with_inclusive_dates_and_no_writes` includes a voided expense and posted income, asserts category totals/percentages, and reconciles with flows. Transfers and openings are excluded by the expense-kind predicate. |
| Monthly flows | `backend/crates/api/src/monthly_flows.rs` selects only posted income, actual interest and expenses. `monthly_history_zero_fills_clips_edges_and_reconciles_exact_currency_flows` seeds a transfer, opening and voided expense in December, asserts zero December flows and reconciles every month with the period report. |

All three test groups also check read-only behavior. Approved semantics are in
`docs/product/flow-reports.md`, `expense-distribution.md`, `monthly-flows.md`
and `dashboard-metrics.md`.

## In scope

Execute existing public HTTP report suites against disposable local PostgreSQL;
record evidence and update only P09.07 after successful verification.

## Out of scope

Changing account balance/history semantics, removing transfers or openings from
the ledger, changing interest classification, migrations, new endpoints, and
P06.10 or other roadmap checkboxes. Transfers and openings still affect account
balances; excluding them from realized income/spending must not erase postings.

## Constraints and invariants

Use synthetic data and the existing isolated database harness. Keep income and
spending based on current posted business events, not compensating movements.
Actual credited interest remains included in realized income under the approved
report policy; forecasts do not become realized income.

## Steps

- [x] Inspect filters and representative public HTTP assertions.
- [x] Run all three relevant report test groups.
- [x] Record evidence and mark P09.07 locally verified.

## Acceptance criteria

- [x] Transfers and openings do not increase realized income or spending.
- [x] Voided events do not contribute to period or monthly flows or categories.
- [x] Existing public HTTP suites pass against isolated PostgreSQL.

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
| Three disposable PostgreSQL report targets | Passed: 3 harness targets in 36.559 seconds; 3 flow + 4 distribution + 4 monthly Rust tests (11 total) |
| Repository validation and whitespace | Passed after documentation changes |

## Risks, assumptions, and decisions

Verification/documentation only: existing coverage is reused instead of adding
duplicate implementation. RED→GREEN does not apply because no runtime behavior
or tests change. Formatting, Clippy and the complete Cargo workspace suite are
not rerun for passive documentation; relevant PostgreSQL suites run explicitly.

## Completion notes

Publication pending. Keep this record in `active/` until merge. No commit, push
or PR is performed. P09.06 work is preserved on the current local branch.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `4d348d3 test(reports): verify reporting invariants and profile queries (P09.06-P09.10) (#103)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
