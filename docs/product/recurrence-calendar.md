# Recurrence calendar policy (P07.01)

Monthly, quarterly and annual recurring expenses use the last valid day of a
scheduled month when their nominal day does not exist. Preserve the original
nominal day: a short month must never permanently shift the schedule.
This is an accepted product policy, not an implemented recurrence engine.

## Calendar rule

- Keep the rule's original calendar anchor and nominal day, not merely the last
  generated due date. Monthly, quarterly and annual cadences advance by one,
  three and twelve calendar months respectively from that anchor.
- For each scheduled month, use `min(nominal_day, days_in_target_month)`.
  The adjustment changes only that occurrence's due date, not the rule's anchor.
- February has 28 days in ordinary years and 29 in Gregorian leap years. Annual
  February 29 schedules fall on February 28 in ordinary years and return to
  February 29 in leap years.
- Do not skip the scheduled month or overflow into the next month. A nominal 30
  does not become 31 merely because an earlier occurrence fell on a month end.
- Due dates remain civil dates in the rule's explicit IANA zone, as defined by
  the [date policy](dates-and-time-zones.md#recurrences-and-civil-time-changes).
  A scheduled occurrence remains planned; this date adjustment does not post an
  expense or change an account balance.

Weekly recurrence, rule edits, start/end inclusion, lifecycle and generation
idempotency are outside this task. This policy introduces no configurable
skip-month or roll-forward alternative.

## Acceptance examples

These specify the future calculator's expected output. They are not executed
Rust recurrence tests. Each sequence derives from the original anchor, never
from a previously adjusted occurrence.

| ID | Cadence and anchor | Expected subsequent due dates |
| --- | --- | --- |
| RC01 | Monthly, `2025-01-29` | `2025-02-28`, `2025-03-29` |
| RC02 | Monthly, `2024-01-29` | `2024-02-29`, `2024-03-29` |
| RC03 | Monthly, `2025-01-30` | `2025-02-28`, `2025-03-30`, `2025-04-30`, `2025-05-30` |
| RC04 | Monthly, `2024-01-30` | `2024-02-29`, `2024-03-30` |
| RC05 | Monthly, `2025-01-31` | `2025-02-28`, `2025-03-31`, `2025-04-30`, `2025-05-31` |
| RC06 | Monthly, `2024-01-31` | `2024-02-29`, `2024-03-31` |
| RC07 | Quarterly, `2023-11-30` | `2024-02-29`, `2024-05-30`, `2024-08-30`, `2024-11-30` |
| RC08 | Quarterly, `2024-11-30` | `2025-02-28`, `2025-05-30` |
| RC09 | Quarterly, `2025-01-31` | `2025-04-30`, `2025-07-31`, `2025-10-31`, `2026-01-31` |
| RC10 | Annual, `2024-02-29` | `2025-02-28`, `2026-02-28`, `2027-02-28`, `2028-02-29` |
| RC11 | Annual, `2025-04-30` | `2026-04-30`, `2027-04-30` |
| RC12 | Annual, `2096-02-29` | `2097-02-28`, `2098-02-28`, `2099-02-28`, `2100-02-28` |
| RC13 | Annual, `1996-02-29` | `1997-02-28`, `1998-02-28`, `1999-02-28`, `2000-02-29` |

## Delivery boundary

P07.01 defines this policy only. P07.02 now defines the
[recurrence model and lifecycle](recurrence-lifecycle.md), also as specification only.
P07.04 implements the pure calculator; P07.05 and P07.06 add executable cadence,
leap-year and year-transition regressions. Those runtime tasks remain pending.
See [ADR-034](../architecture/decisions/034-anchored-recurrence-month-ends.md)
for the decision and tradeoffs, and the [master plan](../plans/master-plan.md#phase-07--recurring-expenses-and-due-dates)
for implementation status.
