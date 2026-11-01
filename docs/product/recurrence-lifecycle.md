# Recurring expense model and lifecycle (P07.02)

A recurring rule plans expenses; an occurrence represents one scheduled payment.
Neither is a posted financial transaction. Only explicit confirmation of an actual
payment creates an expense. This specification defines the model and lifecycle;
this document itself does not implement runtime behavior. Storage and the pure
calculator are delivered separately; the [P07.07 rule API](recurrence-rules.md)
implements rule mutations; [P07.08](recurrence-generation.md) implements due generation.
Workers and posting remain pending.

## Rule object

These are conceptual domain fields, not a published JSON or database schema.
Reuse the existing exact money, currency, business-date and account policies.

| Field | Meaning and invariant |
| --- | --- |
| Identity and owner | Stable rule identity and authenticated owner; ownership is server-enforced. |
| Expense terms | Positive exact amount, SEK/EUR/USD currency, optional expense-compatible built-in category and description. This is an expense rule, not a transfer or ordinary-income rule. |
| Optional account | An owned account in the expense currency, or explicitly unassigned. Unassigned planned payments remain visible without affecting any account. |
| Cadence | Weekly, monthly, quarterly or annual. Weekly uses seven calendar days; month-based cadences follow the original calendar anchor. |
| Start and calendar anchor | Civil start date and original nominal calendar day; the start is the first scheduled date. Short-month adjustment never replaces the anchor. |
| Optional end | Inclusive final eligible scheduled date; absent means no scheduled end. An end before the start is invalid. |
| Rule time zone | Explicit IANA identifier belonging to the rule, not a live reference to the profile zone. |
| State | `active` or `paused`; generation eligibility is separate from the state of any occurrence. |
| Generation progress | Enough durable progress to avoid reissuing an existing occurrence or backfilling paused periods. P07.08 owns the persistence algorithm. |
| Audit metadata | Server-owned UTC creation/modification instants, distinct from scheduled dates. |

The next due date is the next eligible ungenerated slot under the rule's calendar,
end and pause history, not the oldest unpaid occurrence. It is unavailable while
paused or when the schedule is exhausted. A future stored cursor is generation
metadata, not a new calendar anchor.

The [calendar policy](recurrence-calendar.md) supplies missing-day and leap-day
adjustment. Dates use the [date/time-zone policy](dates-and-time-zones.md).

## Occurrence object

| Field | Meaning and invariant |
| --- | --- |
| Identity, rule and owner | Stable occurrence identity; the rule and all references have the same owner. |
| Scheduled date | Original calendar slot, immutable even when the payment is postponed. Logical uniqueness is `(rule_id, scheduled_date)`. |
| Due date | Effective planned due date; initially the scheduled date. Postponing changes this date, not identity or the rule's anchor. |
| Planned expense terms | Amount, currency, optional category, description and optional account captured when generated; later rule edits do not rewrite them. |
| State | `planned`, `confirmed` or `skipped`. |
| Confirmed transaction | Exactly one linked expense for a confirmed occurrence; none for planned or skipped occurrences. Actual payment terms belong to that expense. |
| Audit metadata | Server-owned UTC instants separate from the scheduled, due and actual payment dates. |

An overdue occurrence is still `planned`; overdue is a derived presentation
condition, not a new posting state. Postponed is also not a separate state.
Two distinct scheduled occurrences can have the same effective due date without
being merged. Existing occurrence terms are planning data, not financial-audit
value snapshots.

## Rule transitions and preservation

| Action | Rule result | Existing occurrences |
| --- | --- | --- |
| Create | Starts `active` with the explicit calendar and terms. | Generation creates separate planned occurrences, never financial postings. |
| Pause | `active` → `paused`; stop new generation. | All created items remain, including future and overdue planned items; do not hide, skip or delete them. |
| Resume | `paused` → `active`; continue at the next eligible original-calendar date. | Preserve existing items; do not generate missing slots from the paused interval. |
| Edit | Preserve state; changed terms apply only to new occurrences. | Do not rewrite planned, skipped or confirmed items. Individual planned changes require an explicit occurrence action. |
| Reach end | No generation beyond the inclusive end; schedule exhaustion is derived, not payment completion. | Outstanding planned items remain actionable; confirmed/skipped history remains. |

Pause/resume does not reset the original anchor. Treat pause intervals as civil
date intervals in the rule's zone: exclude ungenerated slots from the pause date
up to, but not including, the resume date. Resume may use a slot due on its local
resume date. Already-created items are retained regardless of that interval.
Repeated pause/resume must preserve all excluded intervals; a later worker must
not backfill them. Ordinary worker downtime while active is not a user pause.

An edit does not regenerate, duplicate or replace existing slots. Schedule and
zone edits require explicit review; a profile-zone change is not a rule edit.
The [P07.07 rule API](recurrence-rules.md#edit-boundary-and-pause-history)
advances inclusive coverage for actual start/cadence/zone edits past the edit
day and prior covered periods. P07.08 must honor that boundary without
retroactively materializing revised slots. New/reassigned account selection
rejects archived accounts, while existing references survive archival.
No rule deletion or automatic removal of existing commitments is introduced.

## Occurrence transitions and accounting

| Action | Transition | Accounting effect |
| --- | --- | --- |
| Generate | Absent slot → `planned`, once. | None. |
| Due date passes or user reviews | `planned` → `planned`. | None; elapsed time is not confirmation. |
| Postpone or explicitly edit planned terms | `planned` → `planned`; identity remains fixed. | None; other occurrences and the rule anchor remain unchanged. |
| Skip | `planned` → `skipped`. | None; remove only that occurrence from pending commitments. |
| Confirm actual payment | `planned` → `confirmed`, linked to one expense. | Create the expense, any account movement, audit and confirmation linkage atomically and idempotently. |
| Retry confirmation | Already confirmed → same confirmed result for the same operation. | No second expense or movement; conflicting intent must not overwrite the original result. |

Confirmed and skipped items cannot be confirmed, postponed or skipped again as a
new operation in this lifecycle. A rule pause or exhausted schedule does not
prevent acting on an already-created planned occurrence. Reopening skipped items
is outside this slice. Corrections/voids of a confirmed expense use the existing
transaction workflow; they do not automatically reopen or regenerate its occurrence.

Confirmation reviews the actual amount, business date and optional account,
independently of the planned terms. Apply the existing verified-owner, account,
currency, category and nonfuture realized-date guards. Rejection changes neither
the occurrence nor financial data. Accountless confirmation creates a reportable
expense without an account movement. A linked account must still be valid for
posting; retaining a planned reference never bypasses an archived-account guard.
Only planned occurrences contribute to pending commitments, separated by currency
and assigned account; actual expenses belong to realized reporting instead.

## Acceptance scenarios for later implementation

These are end-to-end specification cases. P07.07 tests their rule-lifecycle
portions and P07.08 tests generation/preservation. Posting and occurrence actions
still require their later runtime tests.

| ID | Scenario | Required result |
| --- | --- | --- |
| RL01 | Generate the same rule/slot twice. | One planned occurrence, no expense or movement. |
| RL02 | A planned payment becomes overdue. | Still planned and pending, with unchanged posted balances. |
| RL03 | January 31 monthly rule pauses February 1, resumes April 1; February/March slots were not generated. | No February/March backfill; next slot is April 30, then May 31. |
| RL04 | A February occurrence already exists before the RL03 pause. | It remains actionable and pending until explicitly confirmed, skipped or changed. |
| RL05 | Edit the amount from 100 SEK to 120 SEK after one occurrence exists. | Existing occurrence keeps 100 SEK; new occurrences use 120 SEK. |
| RL06 | Change the profile zone while a rule uses Europe/Stockholm. | Rule zone, dates and existing occurrences remain unchanged. |
| RL07 | Postpone January 31 to February 5, then rerun generation. | Same January slot/identity, one planned item; next monthly slot remains February 28/29. |
| RL08 | Confirm a planned accountless payment; retry the same confirmation. | One expense, no movements, occurrence no longer pending. |
| RL09 | Confirm with a foreign, archived or currency-incompatible account. | Reject without changing occurrence, expense, movement or audit state. |
| RL10 | Rule reaches its end while a payment remains planned. | No later scheduled slots; existing unpaid occurrence remains pending and actionable. |
| RL11 | Skip an occurrence, or void a confirmed expense. | No automatic reopening or duplicate generation of that scheduled slot. |
| RL12 | Rule is paused on February 1 and resumed February 28, with an ungenerated monthly slot due February 28. | That slot is eligible on resume; earlier paused slots stay excluded. |

## Implementation boundary

[ADR-035](../architecture/decisions/035-recurring-expense-lifecycle.md) records the
preservation decision. P07.03 owns migrations; P07.04–P07.06 own the calculator and
calendar tests; P07.07 owns rule operations; P07.08/P07.09 own generation and the
worker; P07.10/P07.11 own confirmation and occurrence actions; P07.12 proves the
no-premature-accounting invariant. The [P07.03 storage migration](../architecture/recurrence-storage.md)
and [pure calculator](recurrence-calendar.md#pure-calculator-p0704) are delivered.
The [P07.07 rule API](recurrence-rules.md) implements creation, editing, pause and
resumption; [P07.08 generation](recurrence-generation.md) materializes due planning
slots. Workers and occurrence/payment actions remain pending.
