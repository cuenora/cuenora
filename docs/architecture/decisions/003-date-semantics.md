# ADR-003: Separate business dates from audit instants

- **Status:** Accepted design; runtime implementation pending
- **Date:** 2026-10-24
- **Scope:** P03.03; date, timestamp and named-zone semantics

## Context

Cuenora records dated financial events, including backdated and accountless
entries. A UTC audit instant answers when the record was accepted, while the
business date answers which economic day it belongs to. The roadmap already
separates them; travel, midnight and DST require explicit calendar rules.

The maintainer approved a stable confirmed profile zone, a confirmed device-zone
suggestion during setup, past/today-only realized dates and independent recurrence
zones that change only when the rule is explicitly edited.

## Decision

Use exact date-only business values in the positive four-digit Gregorian range
0001–9999. Audit fields are server-owned UTC instants at microsecond precision,
serialized with a `Z` suffix and six fractional digits. Reject invalid dates,
out-of-range values and client audit overrides without changing financial state.

Derive today from one captured trusted UTC instant and the confirmed profile's
IANA zone. Supply those inputs to deterministic checks; do not read clocks or
ambient device/database/server zones inside domain calculations. Enforce no future
business date on new realized records and explicit date edits. An explicit zone
change must not retroactively rewrite or invalidate existing records.

Store each recurrence's named zone separately; a profile update cannot silently
reschedule it. Persist profile/rule zone identifiers separately from timestamps,
because `TIMESTAMPTZ` does not retain the supplied zone name. Keep time-zone rules
maintained rather than freezing a numeric offset.

The [date specification](../../product/dates-and-time-zones.md) defines exact
formats, boundaries and examples. No library, migration or runtime API is added.
Month-end scheduling, local-time triggers, audit ordering/version mechanisms and
interest calculation boundaries retain their own implementation decisions.

## Alternatives and tradeoffs

| Alternative | Benefit | Reason for this choice |
| --- | --- | --- |
| Derive business dates from creation timestamps | Fewer independent values | Moves backdated entries into the wrong financial period. |
| Follow the device zone automatically | Reflects the current physical location | Travel could silently change defaults and calendar boundaries. |
| Use UTC or a fixed numeric offset for every calendar | Simple conversion | Does not represent the user's confirmed local calendar and changing civil-time rules. |
| Apply profile-zone changes to all recurring rules | One global setting | Silently changes independently configured payment calendars. |
| Support the database's entire date range | Larger range | Positive four-digit dates simplify interchange; wider ranges need an explicit compatibility change. |

## Implementation status

P03.03 is defined and its calendar examples are verified independently. Money
remains the only implemented domain behavior. Future date types, audit ownership,
profile storage, recurrence generation and SQL constraints need their own tests.
