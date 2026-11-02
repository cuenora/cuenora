# Dashboard metrics and payment planning

**Roadmap:** P09.01 · **Status:** Defined; P09.02–P09.05 implemented locally · **Date:** 2026-11-02

Show what has been recorded separately from what may be needed. A planning
comparison is not a bank balance, a payment instruction or permission to spend.
P09.01 defines these boundaries; P09.02–P09.10 implement and verify reporting.

## Metric dictionary

All amounts use exact money and remain separate by currency (SEK, EUR, USD).
Never create an implicit converted grand total.

| Metric | Meaning |
| --- | --- |
| Current recorded balance | Sum of posted account movements, including openings, transfers and compensating postings. Never derived from income minus expenses. |
| Total recorded balances | Sum of owned account balances in one currency; preserve account-level visibility. Archived accounts must not silently lose their retained balances; identify them separately from active accounts. |
| Savings | Posted balances of savings accounts, a subset already included in the corresponding currency total, not extra money or net flow. |
| Realized income | Current non-voided ordinary income and actual credited interest with business dates in the selected reporting period, including accountless ordinary income. |
| Realized spending | Current non-voided expenses in that period, including accountless expenses and confirmed recurrence payments exactly once. |
| Net flow | Realized income minus realized spending in the same period and currency; not a balance or an amount available to save. |
| Pending payments | Persisted occurrences still `planned`, using their saved amount/account and effective due date. |
| Projected payments | Eligible future recurrence slots not yet persisted, calculated read-only from current rule terms. Not actionable occurrences or realized expenses. |
| Planning difference | Current account balance minus that account's pending payments minus projected payments through the selected planning date. |
| Forecast interest | Separately labeled estimate with its own explicit range, assumptions and limitations; never added to balances, realized income or the planning difference. |

Transfers, account openings, voided events, planned occurrences and all forecasts
are excluded from realized income/spending. Corrections use the current effective
event once, not every revision or compensating movement as another expense.
Actual interest counts as realized income even if its receiving account is normal.
Category distributions use realized expenses, including uncategorized amounts;
zero total spending has no percentage distribution, rather than a division by zero.

## Dates and snapshot boundaries

The reporting period and planning cutoff are independent, explicitly labeled civil
dates. Reporting includes both period endpoints and uses business dates, not audit
timestamps. Account balance history belongs to P09.05, not the current balance card.

The maintainer approved **the last day of the current month** as the initial
planning cutoff. Derive it from one trusted instant in the confirmed profile IANA
zone, following the [date policy](dates-and-time-zones.md); never silently use the
device/server zone. Explicit selection wins and is preserved on return. A missing
confirmed zone requires confirmation, not a guessed default. The wireframes'
October dates remain examples, not runtime defaults.

For selected date D, include pending occurrences with `due_on <= D`, with no
lower bound: overdue unpaid items must not disappear at month start. Include the
cutoff day; exclude later due dates. Compare civil dates directly, without turning
rule dates into profile-zone midnight instants. Rule-local today still determines
which slots are future; profile-zone today only defines the default cutoff.

Use a consistent current read snapshot and identify its as-of context. Selecting
a past cutoff filters obligations; it does not rewind the current balance or
reconstruct a historical state of pending items. Label the comparison accordingly.

## Pending and projected payment boundaries

The maintainer approved subtracting **both** components, displayed separately.
Reuse the [recurrence calendar](recurrence-calendar.md), pause/end/edit policies
and [generation eligibility](recurrence-generation.md), without calling generation
or changing its through-today horizon during a report read.

- Persisted occurrences take precedence by `(rule_id, scheduled_on)` in every
  state. Never project a second copy of a planned, confirmed or skipped slot.
- Pending items use saved terms and effective `due_on`, not the rule's current
  terms or original scheduled date. A postponed item beyond D is excluded, and
  its original slot must not reappear as a projection. Distinct slots sharing a
  due date remain distinct payments.
- Future projections cover eligible ungenerated slots after today in each rule's
  zone through D inclusive. Apply the original calendar anchor, start/end bounds,
  current state, pause exclusions and generation/edit coverage. Paused rules have
  no new projections; existing planned items remain pending even after pause/end.
- Use current rule amount, currency and optional account for projections. Do not
  predict unrecorded edits, resumes, transfers, income or interest receipts.
- Ungenerated due backlog is not a future projection. Missing generation coverage
  or other unavailable inputs must be surfaced as incomplete planning, never as
  a complete zero or an authoritative difference. P09.05 must verify coverage.
- Generating a projected slot later replaces its projection with one pending item;
  it must not increase the combined commitment total solely by materialization.

## Per-account presentation and safety

For each account/currency, show recorded balance, pending subtotal, projected
subtotal, cutoff, and the formula. A negative result is an estimated shortfall;
zero means the recorded balance exactly covers these known commitments, not that
payment is guaranteed. Do not offset a shortfall with another account's surplus.

Unassigned pending items and projections remain visible with separate totals per
currency, outside every account difference. Accountless realized expenses still
count in spending but do not reduce any account balance. Preserve archived-account
assignments visibly; do not hide them, reassign them or imply they authorize posting.

A genuine empty set yields exact zero. Failed, partial or unavailable reads are
not empty sets: show unavailable/incomplete amounts and do not present a complete
planning difference. Pagination must not turn one visible page into a full total.
All future queries must enforce authenticated ownership, retain exact currencies,
and perform no financial or occurrence writes.

## Acceptance examples for reporting implementation

These are specification cases, not evidence that reporting endpoints exist.
Unless stated otherwise all examples are in SEK with an explicit future cutoff D.

| Case | Required result |
| --- | --- |
| A has 300; pending 200; future projected 600 through D | Show 200 pending, 600 projected and difference −500. Another account's 2,000 does not cover A implicitly. |
| Transfer 500 from B to A in that example | A balance 800, difference 0; B balance falls by 500; income/spending unchanged. |
| A projected 600 slot becomes a planned occurrence | Pending becomes 800, projected 0, difference remains 0; no balance/spending effect. |
| Confirm that 600 payment from A exactly once | Balance 200, pending 200, difference 0; spending increases by 600 once. |
| Planned 50 overdue, 70 due on D, 90 due after D | Pending 120; exclude the 90. Include the overdue item regardless of reporting period start. |
| Postpone the 70 beyond D, then skip the overdue 50 | Pending 0; neither original slot is projected again. |
| Pause/end a rule with an existing unpaid item | Keep that item pending when due by D; no new slots from paused/exhausted schedules. |
| Unassigned pending 40 and projected 60; accountless realized expense 25 | Show unassigned subtotals 40 and 60; account differences unaffected; spending includes 25. |
| Total SEK 2,300 includes savings 1,000; EUR 100 also recorded | Show savings as subset; no 3,300 SEK or mixed-currency total. |
| Income 1,000, actual interest 4, expenses 300, forecast interest 9 | Income 1,004, spending 300, net flow 704; forecast 9 separate and excluded from planning. |
| Profile-zone today is 2026-02-10 / 2028-02-10 | Initial cutoff is 2026-02-28 / 2028-02-29; explicit selected date overrides it. |
| Generation is behind / read fails / only one page loaded | Do not report complete zero obligations or a complete planning difference. |

## Implementation boundary

No endpoint, JSON field, database migration, dashboard UI or projection engine is
introduced by this specification. P09.02–P09.04 implement realized reports;
P09.05 owns history, pending/projected totals and differences; P09.06–P09.10
verify currency, exclusion, accountless, filtering and performance behavior.
