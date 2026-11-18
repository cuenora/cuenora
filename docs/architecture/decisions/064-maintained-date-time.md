# ADR-064: Maintained date/time library

- **Status:** Accepted; implementation and scoped integration verification passed
- **Date:** 2026-11-18
- **Roadmap:** Preserve existing business-date, time-zone and financial-date runtime contracts

## Decision

Replace the unmaintained `chrono` and `chrono-tz` dependencies with Jiff. Use
Jiff's `civil::Date` for Gregorian date validity and arithmetic, `Timestamp` for
captured UTC instants, and its IANA time-zone database for named-zone conversion.
Enable `tzdb-bundle-always` and explicitly use
`TimeZoneDatabase::bundled()` for named-zone validation and conversion, so
production behavior retains compiled-in IANA availability rather than depending
on host zoneinfo or Jiff's host-preferred default database.

Keep `BusinessDate` as the public domain boundary: exact ASCII `YYYY-MM-DD`,
years 0001–9999, impossible dates rejected, and the same display representation.
Keep endpoint request/response formats and errors unchanged. Profile and
recurrence zones remain exact-case IANA identifiers and recognized aliases;
offset strings, non-database POSIX rules, unknown names, whitespace and control characters
remain rejected. Reuse the existing `today_at` helper and preserve captured
instant, local-date, recurrence anchor/month-end and savings day-count behavior.

## Tradeoffs and boundaries

Bundling the IANA catalog preserves host-independent operation but updates only
when the dependency is rebuilt with a newer database. Strict exact-case lookup
against the bundled Jiff database inventory preserves the previous acceptance
policy, which is narrower than Jiff's case-insensitive lookup. Jiff is used only as the
date/time implementation; no endpoint, database schema, financial policy, or
calendar rule changes are included.

ADR-024 documents the historical Chrono-based implementation decision and is
retained unchanged. This record supersedes only its library choice, not its
financial or API contract.
