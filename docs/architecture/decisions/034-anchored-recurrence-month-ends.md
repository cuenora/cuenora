# ADR-034: Anchored recurrence month-end adjustment

- **Status:** Accepted; policy only, runtime pending
- **Date:** 2026-10-31
- **Roadmap:** P07.01

## Context

Monthly, quarterly and annual recurring expenses may target days 29, 30 or 31
that do not exist in a scheduled month. February 29 also needs an annual policy.
The date policy deliberately left this decision to Phase 07. Recurrences remain
planned until an actual payment is confirmed.

## Decision

Use the scheduled month's last valid day when the nominal day is absent.
Preserve the original calendar anchor and nominal day for every later period;
do not advance from an adjusted due date. Apply this policy to monthly,
quarterly and annual schedules, including February 29.

For example, a monthly January 31 schedule uses February 28/29 and then March 31.
An annual February 29 schedule uses February 28 in ordinary years and returns
to February 29 in leap years. Gregorian century rules still apply.

See the [calendar policy and acceptance examples](../../product/recurrence-calendar.md)
for the normative rule. No database, API or recurrence engine changes are part
of this decision; model, lifecycle and calculator work remain pending.

## Tradeoffs

Skipping invalid dates would omit a month's planned payment; overflowing into
the following month would move it outside its scheduled period. Last-valid-day
adjustment avoids both, but due dates vary in short months. Preserving the anchor
prevents permanent drift and requires future calculation/storage to retain that
intent rather than only the previous adjusted date. Configurable alternatives
are not introduced in this slice.
