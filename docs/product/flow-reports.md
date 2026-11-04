# Realized flow report (P09.02)

`GET /v1/reports/flows?date_from=2026-11-01&date_to=2026-11-30` returns
one full-period, owner-scoped snapshot of income, expenses and net flow.
Requires a valid bearer session and verified email. It does not require a profile
zone because the caller supplies both business-date bounds; no default is inferred.

## Contract

Both strict `YYYY-MM-DD` dates are required, ordered and inclusive. Same-day,
empty and future periods are valid; future dates do not create forecasts.
Unknown/duplicate parameters, nonempty bodies and queries over 1024 bytes reject.
There are no pagination, currency, category or account filters in this slice.

```json
{
  "date_from": "2026-11-01",
  "date_to": "2026-11-30",
  "currencies": [
    {"currency": "SEK", "income": "1004.00", "expenses": "300.00", "net_flow": "704.00"},
    {"currency": "EUR", "income": "0.00", "expenses": "7.99", "net_flow": "-7.99"},
    {"currency": "USD", "income": "0.00", "expenses": "0.00", "net_flow": "0.00"}
  ]
}
```

The three rows always appear in SEK/EUR/USD order; absent activity is exact zero.
Totals are decimal strings, can exceed a single transaction's i64 minor-unit
range, and are never combined across currencies. `net_flow = income - expenses`.

Use current posted business events, not account movements: ordinary income and
actual interest contribute to income; expenses contribute to spending. Include
accountless events and prior events on archived accounts. A confirmed recurring
expense counts once as its ordinary posted expense. Exclude transfers, openings,
voided events, planned payments and all projections. Corrections use the current
event amount/date/state, never sum revisions or compensating postings as new flow.

Reads use repeatable-read read-only transactions with 2-second lock and 5-second
statement timeouts. Database failure returns safe `503 flow_report_unavailable`,
not zero or partial totals. Responses retain request IDs and `Cache-Control: no-store`.
No occurrence, event, posting, audit or idempotency state is changed.

See [metric definitions](dashboard-metrics.md), the
[OpenAPI contract](../../contracts/openapi.yaml), and the
[execution record](../plans/completed/2026-11-02-flow-reports.md).
Category, monthly and balance-history queries remain later roadmap work.
