# Execution plan: Exact daily savings interest

- **Status:** Active
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P08.04
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Provide a pure, exact daily accrual primitive, not a complete forecast.

## Existing state and evidence

Savings policy already defines closing balance, Actual/Actual calendar-year day
count and no daily rounding. Money is i64 minor units. Configuration accepts
nominal percentages up to 1024 characters with six fractional places.

## In scope

- Explicit nominal percentage parsing and rational daily accrual with currency.
- Public-interface regression tests, dependency lock and ADR-043.

## Out of scope

P08.05+ payout forecasts, capitalization, histories, HTTP and posting.

## Constraints and invariants

No float arithmetic, daily rounding, runtime I/O or posted-balance changes.
Existing financial behavior remains unchanged; only new domain exports are added.

## Steps

- [x] Inspect policy and numeric bounds; confirm arbitrary precision with maintainer.
- [x] Observe new tests fail before implementing the API, then pass.
- [x] Complete workspace checks and independent dependency verification.

## Acceptance criteria

- [x] Exact policy examples, repeating fractions, 365/366 and Gregorian century rules.
- [x] Zero rates, negative balances, full accepted rate length and currency preservation.
- [x] Invalid rate grammar rejected without rounding or normalization.
- [x] Documentation and independent verification complete.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| New public test before implementation | Missing API fails | Unresolved imports and dependency | RED |
| `cargo test -p cuenora-domain --test savings_interest` | Pass | Six tests passed | GREEN |
| Workspace format, Clippy, tests | Pass | All passed; PostgreSQL tests remain ignored | PASS |
| `python3 scripts/check_repository.py` | Pass | Passed | PASS |
| Independent isolated probes | Pass | Six public tests plus three independent groups; full domain suite and offline dependency checks passed | PASS |

## Risks, assumptions, and decisions

Risk: item 5 (new numeric dependencies and lockfile). ADR-043 records the approved
choice. RDD remains globally off. No existing command or shared behavior is
changed, so command differential checks are not applicable. No database behavior
changes; ignored PostgreSQL suites need not run for this pure calculator.

## Completion notes

Local feature branch `feat/p08-04-daily-interest`; no remote authorization.
Maintainer authorized one PR without an issue, using the current gh session and
SSH push. Preserve pre-existing untracked `.atl/`.

Verification completed locally. Independent verification found no blockers.
External vulnerability audit was not run. No database suite was run because
no persistence behavior changed. P08.05+ remain pending.
