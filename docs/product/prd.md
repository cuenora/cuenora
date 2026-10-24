# Cuenora product requirements

**Version:** 0.1 · **Date:** 2026-10-22 · **Roadmap:** P00.10
**Status:** Initial product baseline; financial features are not implemented.

## Problem and primary user

Cuenora helps people understand where their money goes, how much they spend,
and what remains for saving. The maintainer is the only current user: money is
spread across multiple bank accounts, payments leave different accounts, and an
account can lack funds for a scheduled payment even when another has enough.

The first product should make each account's recorded position and upcoming
commitments visible together, reducing the need to reconstruct that picture from
separate bank applications. Without bank synchronization, its usefulness depends
on keeping the records up to date. This is first-user evidence, not external
market validation. See the [target-user definition](target-user.md).

## Intended outcomes

- Understand recorded balances per account and currency.
- Identify which account needs funding for upcoming payments.
- Record actual transfers without mistaking them for income or spending.
- Understand spending, realized net flow, and savings as distinct measures.

The [ten prioritized stories](user-stories.md) express value priority. The
[backlog](backlog.md) maps them to implementation dependencies and acceptance
examples; neither document asserts implementation completion.

## MVP requirements and acceptance

| ID | Requirement | Acceptance evidence to implement |
| --- | --- | --- |
| R-01 | Authenticate users and isolate their accounts and financial records. | Server-side ownership checks reject access and relationships across users; release case FIN-012 and Phase 04 tests. |
| R-02 | Create normal and savings accounts in SEK, EUR, or USD, with explicit opening movements and balances derived from posted movements. | EX-01; opening balances are not income and savings is not counted twice. |
| R-03 | Record dated, categorized income and expenses, with optional account assignment and auditable corrections. | EX-02–EX-04; accountless entries affect reports but not balances; valid assignment applies once without changing the business date. |
| R-04 | Record atomic, idempotent transfers between distinct owned accounts, including manual cross-currency transfers. | EX-05–EX-07; required sent/received amounts determine postings; optional rate cannot override them; failures leave no partial balance effect. |
| R-05 | Schedule recurring expenses with due dates and optional source accounts; explicitly confirm actual payments. | EX-08; planned or overdue occurrences have no realized effect; repeated confirmation posts once. |
| R-06 | Show per-account recorded balances alongside pending payments through a selected date, with an explicitly labeled planning difference. | EX-08; a 300 SEK account with an 800 SEK pending payment shows a −500 SEK planning difference while its posted balance remains 300 SEK. Unassigned payments remain visibly unassigned. |
| R-07 | Show income, spending, net flow, category breakdowns, and savings totals, grouped by currency. | EX-01–EX-06 and EX-10; transfers and openings are excluded from income/spending, accountless entries included, and no implicit FX total is shown. |
| R-08 | Restrict savings contributions to transfers after opening; direct savings income is only actual credited interest. | EX-05 and EX-10; reject ordinary-income creation, edits, and reassignment into savings without changing data. |
| R-09 | Allow savings with no interest or effective-dated variable annual rates and configurable payout schedules; separate forecasts from credited payments. | EX-09; actual payments post exactly once in the receiving account; forecasts never post automatically. Daily, monthly, quarterly, and annual schedules are planned. Calculation policies must be specified before implementation. |
| R-10 | Deliver native Android and iOS experiences with shared reusable client logic and consistent financial semantics. | Phases 10–13: shared contract fixtures, platform tests, accessibility checks, and shared Maestro flows; each platform still requires its own build proof. |

All EX references identify the [ten numerical worked examples](worked-examples.md).
FIN references identify the master plan's
[release acceptance cases](../plans/master-plan.md#8-acceptance-examples-required-before-public-release).

## Accounting and quality requirements

- Use exact arithmetic and explicit currencies; never binary floating-point money.
- Keep business dates distinct from UTC audit timestamps. Preserve auditable
  corrections, reversals, and account history.
- Validate input and enforce ownership on the server; never trust submitted
  user ownership or client balances. Keep sensitive data out of logs and fixtures.
- Match API contracts, clients, and examples. Verify persistence against isolated
  PostgreSQL and test failed writes, retries, and authorization boundaries.
- Present loading, empty, validation, and error states accessibly. The MVP is
  online-first; offline synchronization is not promised.
- Preserve the roadmap's security, backup, and restore verification before release.

These requirements follow the [master plan](../plans/master-plan.md),
[agent instructions](../../AGENTS.md), and
[initial stack decision](../architecture/decisions/001-initial-stack.md).

## Release boundaries

The MVP records user-maintained information; it does not connect to banks, execute
payments, retrieve exchange rates, or automatically convert totals. Reminders
remain later work, so an in-app funding comparison requires opening the app.

Budgets, savings goals, reminders, export, and localization remain in v1.
Offline-first synchronization, imports, attachments, shared finances, automatic
FX/history, reconciliation, and open banking remain later work. Receipt OCR, AI,
and real-money investment features are outside the MVP. No additional release
commitments or deadlines are introduced here.

## Specifications still required before implementation

| Topic | Required specification | Roadmap owner |
| --- | --- | --- |
| Money | [Implemented and locally verified](money.md): scale, bounds, rounding and invalid-precision rejection in domain value types. Financial application use cases remain pending. | P03.01–P03.02 |
| Dates and recurrence | Time zones, month-end/leap-day behavior, and occurrence boundaries. | P03.03; P07.01–P07.06 |
| Transfers | Optional rate precision, direction, discrepancy presentation, and idempotency semantics. | P06.01; P06.07 |
| Savings | Day-count, balance timing, effective-date boundaries, capitalization, rounding, and supported nominal-rate interpretation. Unidentified rate terms must not generate a forecast. | P08.01 |
| Metrics | Exact period boundaries and inclusion rules for pending payments and planning differences; never label the result guaranteed safe-to-save money. | P09.01 |

These are explicit implementation prerequisites, not inferred bank conditions.
No provider-specific interest formula is approved by this PRD.

## Completion and validation

P00.10 is complete when this PRD, backlog, and their links and traceability are
verified. Product release requires the roadmap's implementation and acceptance
checks; completing product-definition documents does not complete those phases.
Future validation should first use the maintainer's multi-account workflow without
committing real financial records to the repository. No measured user outcome,
external study, or release readiness is claimed now.

## Supporting decisions

- [Optional account assignment](accountless-entries.md)
- [Currencies and transfers](currencies-and-transfers.md)
- [Savings representation and accounting](savings.md)
- [Financial glossary](glossary.md)
