# Corrected balance history and account planning (P09.05)

Read financial history separately from planning estimates. Both endpoints require
verified bearer identity and use one owner-scoped repeatable-read read-only snapshot.
No balance, recurrence, session or audit writes occur. See [metric rules](dashboard-metrics.md).

## Daily corrected-ledger history

`GET /v1/accounts/{id}/balance-history?date_from=2024-02-29&date_to=2024-03-01`
accepts two strict ordered business dates, at most **366 days inclusive**.
Active and archived owned accounts are supported; missing/foreign accounts return
opaque 404. Explicit history dates need no profile-zone default; future periods
are valid filters, not forecasts. Years 0001–9999 and same-day windows are supported.

Response fields: `as_of`, `account_id`, `currency`, `status`, `date_from`, `date_to`,
`history_basis: current_corrected_ledger`, `opening_balance`, `days`.
`opening_balance` is the sum of all movements strictly before the first date;
chronological daily `{date, balance}` rows include every date and closing balance.
No movements on a day means carry forward, not reset to zero. Before any posting,
the recorded ledger balance is zero; that is not evidence of a historical bank balance.

The maintainer approved **currently corrected business-date history**. Sum all
original, compensating and replacement movements. A correction moving a December
expense to January cancels its old December posting and posts the corrected January
amount; both dates' history is restated. This is not a reconstruction of what the
app displayed before the correction. Never filter movements solely by the current
transaction state: a void is represented by compensating movements, not by erasing
original/reversal rows. Openings, transfers and actual interest affect balances;
accountless events and projections do not. History does not equal realized net flow.

## Current planning through a selected date

`GET /v1/reports/account-planning?through=2026-11-30` accepts one optional strict
inclusive civil date. Without it, use month end in the confirmed profile IANA zone,
from one trusted server instant. Missing profile zone requires confirmation; never
silently default to UTC/device/server zone. Explicit cutoff does not require a default.
Rule-local today still uses each rule's own saved zone and the same captured instant.

Response fields: `as_of`, `through`, `complete`, `accounts`, `unassigned`.
`as_of` is a UTC microsecond timestamp identifying the current snapshot context.
A past cutoff filters obligations; it does not rewind the current account balance
or reconstruct which occurrences were pending in the past.

Each account row has identity/name/type/status/currency plus:

| Field | Meaning |
| --- | --- |
| `current_balance` | All recorded signed account movements, including compensations. |
| `pending_total` | Saved planned occurrence amounts whose effective due date is <= cutoff; overdue included without a lower bound. |
| `projected_total` | Eligible ungenerated future slots through cutoff, using current rule amount/account/currency. |
| `planning_difference` | Current balance minus pending minus projected; null when this account is incomplete. |
| `complete` | False when eligible due slots after authoritative generation coverage remain ungenerated. |

Accounts are ordered by UUID, including archived accounts; archived assignments
remain planning information, not permission to post. Savings is not added twice.
Do not offset one account's shortfall against another account's surplus.

Unassigned rows are always SEK/EUR/USD in that order with `pending_total`,
`projected_total`, `complete`. They remain outside every account difference.
Amounts are exact two-place strings; sums and differences can exceed a single
transaction's i64 minor-unit limit. There is no implicit FX or mixed-currency sum.

## Projection and completeness rules

- Never call generation from a report. Use existing pure `next_occurrence` with
  original anchor/cadence, inclusive end, generation/edit coverage and native pause
  exclusions. Paused rules create no new projections; their saved pending items remain.
  Eligible missing slots strictly before the pause still make coverage incomplete;
  slots from the pause onward are not projected or invented as due commitments.
- Every persisted `(rule_id, scheduled_on)` wins, regardless of state. A postponed
  payment after cutoff must not reappear at its original slot as a projection.
  Confirmed/skipped slots never project again. Pending uses its saved terms, even
  if the rule now has another amount/account or is paused/ended.
- Only ungenerated slots **after rule-local today** are future projections. Eligible
  ungenerated slots on/before today and cutoff represent missing due coverage.
  Mark the current rule account, or its unassigned currency, incomplete. Respect
  authoritative generated/edit coverage; do not invent excluded historical slots.
- Preserve known pending/projected subtotals on an incomplete row, but label them
  incomplete and return null account difference. Other complete accounts can retain
  their own differences. Root `complete` is false if any account/unassigned row is incomplete.
- Materializing an unchanged projected slot later replaces projection with pending,
  without changing combined commitments solely because generation ran.
- Current rule terms are an estimate, not a promise of future edits, deposits,
  transfers, income or interest. A planning difference is not bank availability
  or permission to spend.

## Bounds and failures

History is at most 366 inclusive days. Planning bounds each request to 1,000 owned
accounts, 1,000 recurring rules and 10,000 candidate slots. Exceeding these returns
`422 planning_too_complex`, never truncated success. A shorter horizon can reduce
slot work; resource-count exhaustion requires separately scoped bounded reporting,
not deleting financial history. Missing due coverage is distinct from execution
failure: it is represented by explicit completeness, not a misleading full estimate.

Queries are at most 1024 bytes; unknown/duplicate parameters and nonempty bodies
reject. No extra account/category/currency filters or pagination. SQL lock/statement
limits are 2/5 seconds. Failed reads return safe `503 account_report_unavailable`,
not zero/partial success; request IDs and `Cache-Control: no-store` remain present.

See [OpenAPI](../../contracts/openapi.yaml), [generation](recurrence-generation.md)
and the [execution record](../plans/active/2026-11-02-account-planning.md).
No posting, migration, dependency, chart UI or financial authority is added.
