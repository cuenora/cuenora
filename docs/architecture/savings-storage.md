# Savings configuration storage

**Roadmap:** P08.02, P08.03 · **Status:** Implemented and locally verified · **Date:** 2026-11-01

Migration `0021_savings_storage.sql` adds optional, effective-dated configuration
for savings accounts. It neither enables forecasts nor posts money.
[ADR-042](decisions/042-effective-dated-savings-storage.md) records the storage choice;
[product policy](../product/savings.md#approved-forecast-policies-p0801) defines calculation semantics.

## Two independent history streams

| Table | Stored facts |
| --- | --- |
| `savings_terms` | Account, owner, inclusive `effective_on`, payout frequency, explicit capitalization mode, nominal annual rate basis, calendar-year day count, daily closing balance timing, period-end half-even rounding, UTC creation timestamp. |
| `savings_rate_periods` | Account, owner, inclusive `effective_on`, exact `nominal_annual_percent`, UTC creation timestamp. |

Each row has a UUID. `(account_id, effective_on)` is unique within each stream.
A version applies until the next version in that stream, exclusive; there are no
editable end dates or overlapping explicit ranges. Rates do not reset when
payout frequency or capitalization changes. Future and historical dates may be
stored; dates must be finite Gregorian dates in years 1–9999.

Composite foreign keys reuse `accounts_owner_type` to enforce both ownership and
`savings` account type, including reverse changes to referenced accounts. Native
unique/reference constraints handle concurrent conflicting inserts without
custom read-then-write guards. Owner/account indexes support scoped lookups;
account/date uniqueness supports ordered history lookup.

## Exact rates and history

The maintainer approved at most six fractional **percentage** digits, without
input rounding. `3.123456` means 3.123456%, not a decimal fraction of one. Zero is
valid; negative rates and nonfinite numbers are rejected. Unconstrained PostgreSQL
`NUMERIC` plus a scale check avoids the implicit rounding of `NUMERIC(p,6)`.
Even `3.1234560` is rejected because the submitted numeric scale exceeds six.
There is no new commercial rate cap; future arithmetic must detect overflow.

Updates, deletes and truncation are rejected for both tables. New versions add
rows instead of replacing history. A future correction/removal workflow needs
its own explicit design; this migration provides no destructive rewrite escape.

## Incomplete configuration is not a forecast

No row is inserted for existing accounts. A savings account can have no terms,
no rates, or insufficient effective-date coverage. The streams intentionally do
not require a rate version to reference a particular terms version. A future
forecast must require both applicable versions and must never invent a rate,
capitalization choice, or coverage before the first effective date.

This migration adds no runtime behavior. P08.03 adds the configuration API below;
calculators, forecasts and payments remain separate tasks. In
particular, mid-period payout-frequency transition semantics and reconciliation
of projected versus actual credited interest need application-level handling;
storage history alone does not implement them.

## Verification

Use the existing isolated harness, never an ambient database:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_savings_storage_history_precision_and_ownership \
  PostgresComposeTest.test_savings_migration_preserves_existing_history \
  PostgresComposeTest.test_all_migrations_reproduce_schema_on_fresh_instances -v
```

The tests exercise exact precision, accepted policies and cadence options, owner
and account-type boundaries, immutable histories, schema reproducibility,
unchanged existing records on upgrade, and absence of financial postings.

## Configuration API (P08.03)

`GET` and `POST /v1/accounts/{id}/savings-terms` manage the terms stream;
`GET` and `POST /v1/accounts/{id}/savings-rates` manage the independent rate stream.
Both require a bearer session, current verified email and an owned savings account.
Missing, foreign and normal accounts return the same `404 not_found`. Active and
archived savings accounts retain configuration access; this does not post money.

POST requires an explicit `effective_on` date. Terms also require every policy
field, including a boolean `capitalize`; no policy is inferred. Rates require
`nominal_annual_percent` as an exact nonnegative decimal string with at most six
fractional digits and at most 1024 characters (bounded parsing/storage input).
Examples and the complete wire contract are in `contracts/openapi.yaml`.

```json
{"effective_on":"2026-11-01","payout_frequency":"monthly","capitalize":false,"rate_basis":"nominal_annual","day_count":"actual_calendar_year","balance_timing":"daily_close","rounding":"period_end_half_even"}
```

```json
{"effective_on":"2026-11-01","nominal_annual_percent":"3.123456"}
```

A stream's account/date slot is its permanent request identity. An identical
retry returns the original version and `201`, including its original ID and
rate representation; numeric-equivalent rates such as `2` and `2.0` replay.
Different values at the same date return `409 savings_configuration_conflict`.
An `Idempotency-Key` header is not needed: new dates create new versions. Owner-first
transaction locks serialize concurrent requests with existing financial operations;
insert/read failures roll back. Updates and deletions remain unsupported, including
corrections to future versions. The maintainer approved this limited scope.

GET returns `versions` in ascending effective-date order and nullable `next_cursor`.
Use `limit` (1–100, default 50) and the exclusive date `cursor` for subsequent pages.
Each response is bounded; historical inserts can appear on earlier pages, so a
multi-request history traversal is not an immutable snapshot. Empty streams return
an empty array, never a default rate or policy. Reads never imply forecast readiness.
Unknown/duplicate query fields, nonempty GET bodies and POST queries are rejected.

Historical and future financial dates are accepted without deriving today; no
profile-zone conversion is needed. These operations never touch transactions,
movements, balances or audit records for financial events. Mid-period payout
transition calculation remains deferred to the calculator/forecast tasks.
