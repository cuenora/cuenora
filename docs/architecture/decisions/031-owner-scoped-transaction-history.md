# ADR-031: Owner-scoped recorded transaction history

- **Status:** Accepted for P05.12; own/independent checks passed, commit/native review pending
- **Date:** 2026-10-30

## Context

P05.12 requires paginated transactions and date/category/account filters. The
maintainer explicitly approved all five recorded event kinds. Planned occurrences
are not transactions. Account listing already establishes verified-owner bounded
keyset pagination and short read-only snapshots.

## Decision

Add GET /v1/transactions with the [history contract](../../product/transaction-listing.md).
Reuse bearer identity, strict BusinessDate parsing and exact Money formatting.
Scope every event, cursor and account lookup to the authenticated owner. Category
filters address the global read-only catalog. Never aggregate currencies or
infer balances from event amounts/state.

Use descending `occurred_on,id` keyset pagination rather than offset pagination;
the last owned, filter-matching UUID is the cursor. Include explicit posted/voided
state, all recorded kinds, archived-account history and accountless events.
An account filter matches both sides of transfers; `none` selects accountless
records. Inclusive dates operate on stored business dates, independent of zones.
Verification, reference checks and selection share one repeatable-read read-only
transaction with local two-second lock/five-second statement timeouts. No schema
or posting protocol changes are necessary.

## Tradeoffs

Keyset pages avoid offset drift and unbounded skip counts, but do not promise a
snapshot across requests. A UUID cursor depends on its current stored date; edits
between pages may change traversal. Unknown/foreign/nonmatching cursors reject
with the same safe 400. A filter change should start a new traversal.

Verified email follows existing private account-read policy, unlike the public
reference catalog. No profile zone is needed to read dates already recorded.
Including voided records preserves transparent history; consumers must not treat
this event list as realized totals. Bounded queries can still time out on large
filtered histories; further index tuning needs measured evidence, not a speculative
migration in this slice.

## Verification

Public HTTP tests cover all kinds, exact dual-currency amounts, date ties,
filter composition, authority/rejections, timeouts, unchanged storage and POST
compatibility. Independent security/shared-route verification is required.
