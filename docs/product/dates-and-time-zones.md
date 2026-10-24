# Business dates, audit instants and time zones

**Roadmap:** P03.03 · **Status:** Defined; runtime implementation pending · **Date:** 2026-10-24

Financial records have a user-selected calendar date. Audit events have a
server-generated UTC instant. Keep these concepts separate: entering a January
expense in February must not move that expense into February's spending.
[ADR-003](../architecture/decisions/003-date-semantics.md) records the design.

## Approved product policies

The maintainer approved these four policies individually:

- A confirmed IANA time zone belongs to the profile and defines calendar today.
  Travel or device-zone changes do not automatically change that saved zone.
- Realized income, expenses and transfers may have past dates or today, never a
  future business date relative to the profile zone. Recurrences and forecasts
  may have future dates and remain separate from realized balances.
- Initial setup suggests the device's named zone and requires confirmation. If
  a valid suggestion is unavailable, require manual selection. No global city
  or silent UTC fallback is chosen on the user's behalf.
- Existing recurring rules retain their own zone when the profile zone changes.
  Change a rule's zone only through explicit editing of that rule.

## Representation and validation

These are engineering decisions for future implementations, not implemented fields.

| Concept | Contract |
| --- | --- |
| Business date (`occurred_on`) | Gregorian calendar date; exact ASCII `YYYY-MM-DD`, from `0001-01-01` through `9999-12-31`. No time, offset or zone component. |
| Effective/scheduled dates | Use the same civil-date representation. The no-future rule applies to realized operations, not future rate terms, due dates or forecasts. |
| Audit instants (`created_at`, `updated_at`) | Server-generated UTC instants; canonical interchange `YYYY-MM-DDTHH:mm:ss.ffffffZ`, six fractional digits. Years 0001–9999, seconds 00–59. |
| Audit precision | Microseconds, compatible with the planned PostgreSQL `TIMESTAMPTZ(6)` boundary. Floor finer clock precision to the preceding microsecond; do not silently round into a later date. |
| Named zones | Valid IANA identifiers from the supported time-zone database, including recognized aliases. Store the confirmed identifier separately from audit timestamps. `Etc/UTC` is an explicit valid choice. |
| Invalid zones | Reject unknown identifiers and bare numeric offsets such as `+02:00`. Do not infer a named zone from an offset or a displayed abbreviation. |
| Invalid dates | Reject impossible calendar dates, year zero, out-of-range years, missing zero padding, whitespace, alternative separators and timestamp strings. Do not clamp or roll dates. |
| Errors | Distinguish malformed date, out-of-range date/instant, unsupported zone, unconfirmed zone and future realized date. Reject the operation without changing records or balances. |

The app's four-digit positive-year range is intentionally narrower than database
date ranges and RFC 3339's syntax. API serialization is explicit; do not expose
the database's locale-dependent date/time text output. No date library or schema
is selected by this document.

### Today, defaults and future validation

At the application boundary, capture one trusted UTC instant for the operation.
Resolve that instant in the confirmed profile zone and take its calendar date:

```text
today = calendar_date(captured_utc_instant, confirmed_profile_zone)
```

Use the same captured instant for a default date and its validation, including
when a request crosses midnight. An explicitly selected valid date takes
precedence over the default. The device clock, server operating-system zone,
database session zone and UTC calendar date alone cannot define user today.
Reject missing/unconfirmed/unsupported profile zones instead of guessing.

When creating a realized record or explicitly changing its business date, require
`occurred_on <= today`. Validate on the server; a client date picker is not proof.
Keep pure date checks deterministic by supplying the instant/derived today and
zone explicitly, without reading the system clock inside domain calculations.
If conversion would exceed the supported date range, return an out-of-range error.

A later profile-zone change never rewrites existing business dates or reverses
posted movements. A previously valid record can therefore temporarily have a
date later than today in the new zone. Keep it intact; editing an unrelated
description or account link must not silently change its date or reject it solely
because today changed. An explicit date change follows the current validation rule.

Historical entries group into financial calendar periods using their stored
business dates, not audit timestamps. P09.01 still owns detailed metric boundaries.
This task does not add bank settlement/value-date fields or manual one-off planned
income/expense entries.

### Audit and persistence boundary

Keep the original creation instant unchanged during corrections, linking or
unlinking. Update the last-modified instant only for an accepted state change;
rejected operations cannot change audit fields. Future mutation APIs treat audit
fields as server-owned, read-only inputs. Never derive an audit instant from a
business date or substitute the device clock.

Microsecond precision may give multiple changes identical timestamps, and a wall
clock is not a monotonic sequence. Do not use audit timestamps as unique event
identifiers, idempotency keys or the sole evidence of mutation order. Rejected
client audit overrides must leave both financial and audit state unchanged.

PostgreSQL stores `TIMESTAMPTZ` instants in UTC and does not retain the originally
supplied time-zone name. Persist profile/rule identifiers in their own fields;
the original zone cannot be recovered from that timestamp. Future migrations
must enforce these boundaries and reject unsupported non-finite date/time values.
See [PostgreSQL date/time semantics](https://www.postgresql.org/docs/current/datatype-datetime.html#DATATYPE-DATETIME-INPUT).

### Recurrences and civil-time changes

New recurring rules may suggest the confirmed profile zone; each rule stores an
explicit zone of its own. Profile-zone changes do not propagate into existing
rules, confirmed occurrences or recorded payments. Confirming a payment records
the actual selected business date, independently of its scheduled due date, and
applies the realized-date validation above.

Use maintained IANA rules when resolving instants. A stable zone identifier does
not freeze its UTC offset or daylight-saving rules; do not replace it with today's
offset. Stored business dates remain unchanged when time-zone data is updated.
See the [IANA time-zone database](https://www.iana.org/time-zones).

Due dates are civil dates here; no local time-of-day execution trigger is approved.
A skipped or repeated clock hour does not itself create, post or duplicate a
financial movement. P07 must define schedule generation, invalid month-day/leap-day
policies, edits and deduplication before implementation. P08.01 still owns interest
day counts, interval inclusion and settlement boundaries. Do not infer these
policies from the date representation or from assumed bank conventions.

## Deterministic acceptance examples

These specify future behavior, not executed Rust or database tests. Fixed-input
calendar/zone calculations were independently checked with Python `datetime` and
`zoneinfo`; workflow effects remain future application acceptance tests.

| ID | Input or scenario | Expected result |
| --- | --- | --- |
| D01 | `2024-02-29`, `2000-02-29`, `0001-01-01`, `9999-12-31` | Valid civil dates; serialize without changing their day. |
| D02 | `2026-02-29`, `1900-02-29`, `2026-04-31` | Invalid calendar date; never roll into March/May. |
| D03 | `0000-01-01`, `10000-01-01` | Out-of-range date. |
| D04 | `2026-2-01`, `2026/02/01`, ` 2026-02-01`, `2026-02-01T00:00:00Z` | Rejected business-date input. |
| D05 | Capture `2026-10-24T22:30:00.000000Z`; profile `Europe/Stockholm` | Local time is October 25 at 00:30 (+02:00); today/default is `2026-10-25`, not the UTC day. |
| D06 | Same capture/zone; realized dates October 24, 25 and 26 | First two accepted; October 26 rejected without any financial or audit change. |
| D07 | January 31 expense entered on February 2 | `occurred_on` stays January 31; audit creation is the February UTC instant; the expense remains in January reports. |
| Z01 | Confirm `Europe/Stockholm`, then device changes to `America/New_York` | Profile zone and today calculation remain Stockholm-based. |
| Z02 | Initial valid device zone, unavailable detection, or an unknown zone | Suggest/confirm the valid zone; otherwise require a valid manual choice. Never guess or silently save a suggestion. |
| Z03 | `Europe/Stockholm`, `Etc/UTC`, `Mars/Olympus`, `+02:00` | First two valid named choices; remaining inputs rejected. Recognized IANA aliases remain valid. |
| Z04 | At the D05 instant, explicitly change profile from Stockholm to New York | New today is October 24 (18:30, -04:00); existing October 25 records remain unchanged; new realized October 25 entries are rejected. |
| Z05 | Existing Stockholm recurrence; profile explicitly changes to New York | Recurrence retains Stockholm; no dates or posted movements are rewritten. A rule-zone change requires explicit rule editing. |
| Z06 | Stockholm at `2026-03-29T00:30:00Z` and `2026-03-29T01:30:00Z` | Local times 01:30 (+01:00) and 03:30 (+02:00), both March 29; no 02:30 event is invented. |
| Z07 | Stockholm at `2026-10-25T00:30:00Z` and `2026-10-25T01:30:00Z` | Local 02:30 occurs at two distinct UTC instants (+02:00, +01:00); the business date stays October 25, with no automatic posting or duplicate. |
| A01 | Capture `2026-10-24T22:30:00.123456789Z` | Canonical audit value `2026-10-24T22:30:00.123456Z`; finer precision is floored. |
| A02 | Edit a record created earlier, reject an invalid edit, or try to override audit fields | Creation instant stays fixed; only an accepted state change updates modification time. Rejections leave financial and audit state unchanged. |
| A03 | Two accepted changes within the same microsecond | Equal timestamps are permitted; timestamps cannot identify or order those changes uniquely. |

## Implementation boundary and references

P03.03 completes the definition, not a runtime implementation. Keep tests beside
future date types/use cases and database constraints when those are introduced;
observe RED before behavior changes. No existing Rust code, financial endpoint,
database field or recurrence engine is changed by this specification.

- [RFC 3339 format and Gregorian calendar](https://www.rfc-editor.org/rfc/rfc3339.html#section-5.6); Cuenora deliberately narrows accepted dates and audit output.
- [PostgreSQL date/time types and precision](https://www.postgresql.org/docs/current/datatype-datetime.html).
- [Execution plan](../plans/completed/2026-10-24-date-semantics.md).
