# Execution record: Report currency separation

- **Status:** Verified locally — publication pending
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.06
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Verify “Separate currencies in reports without implicit FX.” Existing P09.02–P09.05
handlers and acceptance tests already implement the requirement. This task records
their verification rather than introducing redundant aggregation or conversion code.

## Existing state and evidence

Baseline: `69d1150` on local `main`.

| Report | Existing public HTTP acceptance evidence |
| --- | --- |
| Flows | `exact_totals_include_boundaries_accountless_and_interest_but_not_other_events` asserts the entire response: distinct SEK and EUR amounts, zero USD, decimal strings and no combined total. |
| Expense distribution | `categories_reconcile_by_currency_with_inclusive_dates_and_no_writes` checks SEK 0.03 separately from EUR 184467440737095516.14, zero USD, same-currency percentages and reconciliation with flows. |
| Monthly flows | `monthly_history_zero_fills_clips_edges_and_reconciles_exact_currency_flows` checks all three currency series, exact values and reconciliation with each currency's flow response. |
| Account planning | `regression_cadences_anchor_exclusions_end_archived_and_currency_projection` checks account-scoped SEK planning and separate unassigned EUR/USD projections. |

The corresponding handlers group or join on currency; distribution percentages
use a currency-partitioned denominator. Approved policies remain in
`docs/product/dashboard-metrics.md`, `flow-reports.md`, `expense-distribution.md`,
`monthly-flows.md` and `account-planning.md`.

## In scope

Run the four existing disposable PostgreSQL targets and record their results.
Update only P09.06 after successful verification.

## Out of scope

FX conversion, new endpoints, migrations, changes to financial behavior, P09.07–P09.10,
and publication. Preserve the pre-existing untracked `.atl/` directory.

## Constraints and invariants

Use synthetic data and the existing local disposable PostgreSQL harness only.
No monetary floating point, implicit FX, or cross-currency grand total.

## Steps

- [x] Map the requirement to existing SQL and public HTTP assertions.
- [x] Execute the four PostgreSQL targets.
- [x] Record observed evidence and update P09.06.

## Acceptance criteria

- [x] Exact totals and monthly series remain separated into SEK/EUR/USD.
- [x] Category percentages use only the matching currency's denominator.
- [x] Account planning retains account currency and separate unassigned amounts.
- [x] Existing public HTTP tests execute, rather than remaining ignored.

## Verification

From the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_flow_reports_against_disposable_postgres \
  PostgresComposeTest.test_expense_distribution_against_disposable_postgres \
  PostgresComposeTest.test_monthly_flows_against_disposable_postgres \
  PostgresComposeTest.test_account_planning_against_disposable_postgres -v
python3 scripts/check_repository.py
```

| Check | Observed result |
| --- | --- |
| Disposable PostgreSQL report groups | Passed: all 4 harness targets; 3 flows + 4 distribution + 4 monthly + 7 planning HTTP tests (18 total); 86.122 seconds |
| Repository validation | Passed after documentation changes |

## Risks, assumptions, and decisions

No runtime or test changes are needed if existing assertions pass. RED→GREEN
does not apply to this verification-only task; no behavior is being implemented.
Cargo formatting, Clippy and the full workspace suite are not rerun for passive
documentation changes; the relevant database suites execute through the harness.

## Completion notes

Local verification only; no commit, push or PR authorized for this small task.
Keep this record in `active/` until merge. No API or financial behavior changed.
