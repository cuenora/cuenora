# ADR-024: Explicit profile zones and atomic dated opening balances

- **Status:** Accepted engineering design; own/independent checks passed, native review pending
- **Date:** 2026-10-29
- **Roadmap:** P05.03; approved P03.03 runtime prerequisite

## Decision

Implement P05.03 and the maintainer-approved minimal profile-zone confirmation
prerequisite, following the existing [date policy](../../product/dates-and-time-zones.md).
`PATCH /v1/me` accepts only an explicit named `time_zone`; bearer identity scopes
the update. No email verification is needed to set a preference, but financial
creation/posting still requires it. Non-null profile zone means explicitly
confirmed through this operation; unknown imported/corrupt values fail closed.

Use [Chrono](https://docs.rs/chrono/0.4.45/chrono/struct.NaiveDate.html) for Gregorian
validity behind a pure, strict `BusinessDate` value, and
[chrono-tz](https://docs.rs/chrono-tz/0.10.4/chrono_tz/) for its compiled named-IANA
catalog and UTC-instant conversion. Explicitly reject the broader date syntax and
year range that Chrono supports. Capture one trusted UTC instant at the application
boundary, not inside the domain. Keep dependency versions/data pinned by Cargo.lock
and review time-zone-data updates through normal dependency maintenance.

Extend creation with optional `opening_balance` containing exact decimal-string
`amount` and explicit `occurred_on`. Preserve the five-field account response and
old omission behavior. An explicitly provided zero amount validates its declared
date/zone and participates in equivalence, but inserts no financial event/posting.
No client owner/category/currency override or implicit date is introduced.

Serialize profile updates and account creation on the existing owner row. Insert
nonzero opening event, signed movement, minimal create audit, account and lifetime
key atomically. Retain no income category or financial values in the audit. Extend
ADR-023 equivalence with normalized amount/date; existing keys receive zero/no-date
without replacing their result. A matching replay checks email but cannot repost
or revalidate an accepted date against a later profile-zone change.

## Tradeoffs and boundaries

Compiled IANA data makes named-zone behavior deterministic across hosts but requires
redeployment to update rules; explicit dependency review is preferable to an implicit
OS-zone fallback. Strict date parsing adds a small project wrapper rather than
reimplementing leap-year/calendar arithmetic. Adding the calendar dependencies
changes the formerly dependency-free domain crate, but keeps it clock/HTTP/SQL-free
in behavior. Domain tests use supplied today values, not wall time.

Explicit dates avoid ambiguous retry defaults across midnight. Zero inputs with a
declared date still require confirmation rather than silently discarding the field.
Per-owner serialization limits create/update throughput but reuses one existing
lock order and keeps retries and profile snapshots coherent. Existing postings and
their economic dates are never rewritten when preferences change.

The [HTTP/storage contract](../../product/opening-balances.md) defines bounds,
errors, no-write guarantees and migration handling. This does not implement email
verification/recovery, a complete profile API, account balance queries or reporting.
