# Monthly realized flow history (P09.04)

`GET /v1/reports/monthly-flows?date_from=2023-11-15&date_to=2024-03-10`
returns monthly income, expenses and net flow by currency. Requires a valid bearer
session and verified email. Explicit dates need no profile-zone default.
This is **flow history**, not account balance history, a cumulative balance, an audit
snapshot or a forecast. Account balance history remains P09.05.

## Calendar contract

Both ordered, strict Gregorian `YYYY-MM-DD` dates are required and inclusive.
The maintainer approved a maximum of **120 intersected calendar months**, counting
partial edge months. January 31, 2015 through December 1, 2024 is 120 months;
extending to January 1, 2025 is 121 and rejects with `400 invalid_request`.
Same-day, leap-day and future periods are valid. Future periods never generate
planned occurrences or forecast payments/interest.

The response object has `date_from`, `date_to`, and `currencies`. Three currency
rows appear in SEK/EUR/USD order; each contains `currency` and `months`.
Each monthly row contains `month` (`YYYY-MM`), effective `date_from` and `date_to`,
`income`, `expenses`, `net_flow`. Months are chronological with no omitted gaps.

For the example request, each currency has these buckets:

| Month | Effective inclusive dates |
| --- | --- |
| 2023-11 | 2023-11-15 through 2023-11-30 |
| 2023-12 | 2023-12-01 through 2023-12-31 |
| 2024-01 | 2024-01-01 through 2024-01-31 |
| 2024-02 | 2024-02-01 through 2024-02-29 |
| 2024-03 | 2024-03-01 through 2024-03-10 |

A clipped edge month must not be labeled as a full-month total. Empty months have
exact `0.00` for all three metrics. Calendar boundaries use business dates, not
UTC audit times or session/device time zones. Supported date extremes are years
0001–9999; monthly stepping must not overflow the externally supported dates.

Example of a single monthly row:

```json
{"month":"2024-02","date_from":"2024-02-01","date_to":"2024-02-29","income":"1004.00","expenses":"300.00","net_flow":"704.00"}
```

## Accounting and reconciliation

Use the [P09.02 flow rules](flow-reports.md): current posted ordinary income and
actual credited interest are income; posted expenses are spending, including
accountless events and confirmed recurrence expenses once. Archived-account events
remain included. Transfers, openings, voids, planned occurrences and all projections
are excluded. Corrections move/update the current event's amount/date; do not sum
audit revisions or compensating postings as additional income/spending.

Amounts are exact decimal strings with two places; aggregate totals may exceed i64
minor-unit transaction limits. No floating point, currency conversion or currency
pooling. `net_flow = income - expenses` per bucket and currency. Each bucket
reconciles with P09.02 for its effective dates, and the complete series sums to the
full selected period for that currency. Separate requests can differ after edits;
reconciliation comparisons require unchanged input data.

## Safety and implementation scope

One repeatable-read read-only snapshot, authenticated owner predicate, 2-second
lock timeout, 5-second statement timeout. Validation bounds the series before SQL;
at most 360 month/currency rows are returned. No writes, session renewal or generation.
Unknown/duplicate parameters, queries over 1024 bytes and nonempty bodies reject.
No implicit dates, account/category/currency filters or pagination in this slice.
Database failures return safe `503 monthly_flows_unavailable`, not partial/zero
success totals. Responses retain request IDs and `Cache-Control: no-store`.

See [OpenAPI](../../contracts/openapi.yaml), [metric definitions](dashboard-metrics.md),
and the [execution record](../plans/completed/2026-11-02-monthly-flows.md).
No UI, migration, dependency, index or balance-history behavior is added.
