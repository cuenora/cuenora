# Execution plan: MVP currencies and manual transfers

- **Status:** Completed
- **Date:** 2026-10-22
- **Roadmap task ID(s):** P00.06
- **Tracking issue / PR:** [#20](https://github.com/cuenora/cuenora/pull/20) — merged 2026-10-22
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Record SEK, EUR, and USD as MVP currencies and support manually recorded
cross-currency transfers with sent and received amounts and an optional rate.

## Existing state and evidence

The master plan and AGENTS.md restrict MVP transfers to the same currency.
US-04 describes only same-currency transfers. No financial behavior is implemented.
The maintainer explicitly approved cross-currency recording and optional rates.

## In scope

- Product decision and examples; consistent roadmap, agent rules, and US-04.

## Out of scope

- Financial implementation, automatic FX, bank execution, and converted totals.

## Constraints and invariants

Preserve atomicity, idempotency, ownership, exact arithmetic, and separate currency
totals. Cross-currency amounts cannot be summed as a zero-sum scalar.

## Steps

- [x] Document the accepted decision and acceptance examples.
- [x] Reconcile current requirements without rewriting historical evidence.
- [x] Validate repository, whitespace, links, and consistency.

## Acceptance criteria

- [x] SEK/EUR/USD, required sent/received amounts, and optional rate documented.
- [x] Same-currency rules preserved; cross-currency restrictions reconciled.
- [x] P00.06 completion supported by documentation checks, not implementation claims.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 scripts/check_repository.py` | Pass | Passed | Passed |
| `git diff --check` | Pass | Passed | Passed |
| Structural and link readback | Consistent current requirements | Read back; restrictions reconciled | Passed |

## Risks, assumptions, and decisions

Actual recorded amounts are authoritative. Rate direction is destination currency
per one source currency unit. Monetary precision and rate validation policies need
specification before implementation. No dependency or schema changes are made.

## Completion notes

Documented the decision and six examples; reconciled AGENTS.md, US-04,
roadmap rules, schema proposal, invariants, and P06 tasks. Repository validation
and whitespace checks passed. This is documentation only; financial behavior
is not implemented and no financial tests were run.

## Merge and archive evidence

- Product documentation was committed as `ee85f1f` and merged through
  [PR #20](https://github.com/cuenora/cuenora/pull/20) on 2026-10-22 at 14:56:35 UTC.
- Merge commit: `312d4789a30b9ad1f2607874bff6c027186533eb`.
- Hosted checks all passed: Repository / Validate, Backend / Rust,
  Backend / Audit, and Backend / Dependency Review.
- The local feature and merged-main trees were verified identical before archival.
- Archived under `docs/plans/completed/` after merge, following the planning convention.
  This closes the documentation work; financial implementation remains pending.
