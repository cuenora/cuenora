# Recurrence planning storage (P07.03)

Migration `0017_recurring_storage.sql` adds `recurring_rules` and
`recurring_occurrences`. It does not create expenses, movements or audit entries,
backfill occurrences, or expose recurrence endpoints. Apply it transactionally
after `0016`; no historical financial row is rewritten.

## Stored boundaries

Rules store exact positive minor-unit expense terms, supported currency, optional
owned currency-compatible account, optional expense category, cadence, start/end,
rule-owned zone and active/paused state. `starts_on` is the calendar anchor;
`nominal_day` derives from it, never from an adjusted occurrence. End is inclusive.
Profile-zone changes do not update rules. Zone storage enforces bounded identifier
shape, not membership in a maintained IANA catalog; future application validation
must enforce the [date policy](../product/dates-and-time-zones.md).

`paused_on` records an open pause and is present exactly when paused. Native
`DATEMULTIRANGE excluded_dates` records bounded completed pause intervals, with
half-open date semantics and no extra extension. PostgreSQL canonicalizes their
union; every excluded date survives repeated pauses. `generated_through` is an
optional inclusive covered-date cursor, not a calendar anchor. P07.07/P07.08 must
serialize rule operations and atomically maintain progress and exclusions;
the migration does not implement pause/resume or generation.

Occurrences retain the original `scheduled_on` slot separately from editable
`due_on`, plus generated expense terms. `(rule_id, scheduled_on)` is unique;
postponement cannot create a new slot. Owners and expense categories are enforced
with composite foreign keys. Account currency is protected in both directions.
A confirmed occurrence must reference an owner-matched expense, and one expense
cannot confirm two occurrences. The reference allows later expense voiding but
prevents changing its kind. Planned/skipped items cannot have that link.

Identity and scheduled slots are immutable; confirmed/skipped occurrence rows
cannot be rewritten. Delete/truncate guards retain recurrence history. Rule
edits do not propagate to existing occurrences. Forward corrections, not destructive
down migrations or deletion, are required for populated installations.

These constraints do not implement authenticated ownership, verified-email gates,
active-account posting checks, schedule eligibility, user-confirmed payment,
confirmation idempotency or atomic expense/movement/audit creation. Those remain
application responsibilities in [P07.07–P07.12](../plans/master-plan.md#phase-07--recurring-expenses-and-due-dates).
See the [lifecycle specification](../product/recurrence-lifecycle.md) and
[execution plan](../plans/active/2026-10-31-recurrence-storage.md) for scope and proof.

[ADR-036](decisions/036-recurrence-planning-storage.md) records the storage tradeoffs.
