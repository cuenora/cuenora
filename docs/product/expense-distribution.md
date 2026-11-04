# Expense distribution by category (P09.03)

`GET /v1/reports/expense-distribution?date_from=2024-02-01&date_to=2024-02-29`
returns full-period realized expenses and category percentages per currency.
Requires a valid bearer session and verified email, just like the
[flow report](flow-reports.md). Both strict business dates are required and inclusive;
ordered same-day, empty and future periods are valid. No implicit profile-zone date.

## Response and reconciliation

The object has `date_from`, `date_to` and `currencies`. Currency rows always appear
in SEK/EUR/USD order, each with `currency`, `expenses` and `categories`.
Category rows contain `category_id`, `code`, `name`, `expenses`, `percentage`.
The seven immutable expense categories appear in UUID order, followed by an
uncategorized row whose ID, code and name are null. Clients supply a localized
uncategorized label; this bucket is distinct from the built-in `other_expense`.
Retain zero category rows so the shape/order is stable, without inventing spending.

All expense amounts are exact two-place decimal strings, including totals beyond
a single transaction's i64 minor-unit range. Category amounts sum to that currency's
`expenses`, which reconciles with P09.02's expense total for the same dates and
snapshot. Separate requests may legitimately differ if a record changes between them.
Never combine currencies or use another currency's total as a percentage denominator.

## Exact percentage policy

The maintainer approved two decimal places with round-half-to-even. Compute
`category minor units * 10000 / currency expense minor units` as an integer ratio,
round to an integer with ties to even, then format it as a two-place percentage
string. Use exact arithmetic; never binary floating point, transaction-size sum
narrowing, or an already-rounded decimal quotient before detecting ties.

- Three categories of 0.01 each yield `33.33` percent each (sum `99.99`).
- Against total 200.00, category 0.01 yields `0.00` percent (exact tie 0.005),
  while category 0.03 yields `0.02` percent (exact tie 0.015).
- A zero category in a nonempty currency has percentage `0.00`.
- If total spending is zero, every percentage is null, not zero or NaN.
- Do not adjust the last category to force a 100.00 total; display rounding context.

For example, a single Housing row can be:

```json
{"category_id":"10000000-0000-4000-8000-000000000004","code":"housing","name":"Housing","expenses":"25.00","percentage":"25.00"}
```

An uncategorized row in an empty currency is:

```json
{"category_id":null,"code":null,"name":null,"expenses":"0.00","percentage":null}
```

## Inclusion and safety

Use only the authenticated owner's current posted expenses, including accountless
expenses and confirmed recurrence payments once. Include recorded expenses even
if their account was later archived. Corrections use current category, amount,
date and state; do not count audit versions or compensating postings as new expenses.
Exclude income (including actual interest), transfers, openings, voided expenses,
planned occurrences and forecast payments/interest. See [metric rules](dashboard-metrics.md).

Reads use a repeatable-read read-only transaction, 2-second lock timeout and 5-second
statement timeout. No posting, generation, session renewal or audit write occurs.
Unknown/duplicate parameters, nonempty bodies and queries over 1024 bytes reject;
there are no account/category/currency filters or pagination in this slice.
Failures never return partial or zero-success totals: database failure maps to safe
`503 expense_distribution_unavailable`. Retain request IDs and `Cache-Control: no-store`.

See [OpenAPI](../../contracts/openapi.yaml) and the
[execution record](../plans/completed/2026-11-02-expense-distribution.md).
No chart UI, category editing, query optimization or other reporting endpoint is
introduced by this slice.
