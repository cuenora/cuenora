# ADR-026: Exact current balances in owner-scoped account pages

- **Status:** Accepted; own/independent checks passed; native review acknowledged
- **Date:** 2026-10-29
- **Roadmap:** P05.05, operation-specific P04.06/P04.10

## Decision

The maintainer approved adding a required `balance` decimal string to each GET
account item, with its existing currency. This extends ADR-025's metadata-only
response; ownership, verified-email gates, pagination and archive visibility stay
unchanged. Creation responses remain metadata-only. A separate balance endpoint
would preserve GET's old shape but require extra requests; no existing client or
historical-filter requirement justifies it in this slice.

Compute each returned account's balance from all its signed append-only postings,
including compensations regardless of current event state. Never join an event's
state to exclude voided postings. No movements means zero; negative balances are
valid. Match authenticated owner, account ID and currency in the movement query.
The existing composite foreign keys also preserve movement/account consistency.

Limit accounts first, then aggregate only that bounded page and its lookahead
row, in the existing read-only repeatable-read snapshot. The database may scan all
movements of those accounts; timeouts bound waits and result memory stays bounded.
This is not a cached balance or a constant-cost query. No index migration is
introduced without measured need.

Use PostgreSQL's exact numeric SUM(bigint), transported as integer text. Validate
returned sums against the existing Money symmetric i64 range, then format through
Money. Do not add postings sequentially in i64: an intermediate sum can overflow
while its final result is valid. No float, numeric dependency or lossy SQL cast is
needed. Reject unrepresentable returned totals as the existing safe unavailable
503; never wrap, clamp, return zero or expose a partial page. The pagination
lookahead is not a returned balance, so it must not fail an otherwise valid page.

## Boundaries

The page's balances and metadata share one snapshot, not a cross-request history.
No as-of filter, aggregate across accounts/currencies, financial write, migration
or stored balance is introduced. Product verification delivery remains pending.
Tests must demonstrate exact sums, compensation semantics, range errors, no writes,
owner/currency isolation and coherent results under concurrent postings.
