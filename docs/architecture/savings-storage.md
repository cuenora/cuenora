# Savings configuration storage

**Roadmap:** P08.02 · **Status:** Implemented and locally verified · **Date:** 2026-11-01

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

There is no configuration endpoint, calculator, payment, idempotency workflow,
or financial-date authority check here. Those belong to P08.03 onward. In
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
