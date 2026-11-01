# ADR-043: Exact daily savings interest

- **Status:** Accepted
- **Date:** 2026-11-01
- **Roadmap:** P08.04

## Context

The approved [savings policy](../../product/savings.md) forbids daily rounding.
Dividing a decimal annual percentage by 365 or 366 can produce a repeating
fraction. Recorded `Money` and finite-scale decimal arithmetic cannot represent
all daily accruals exactly. Configuration already accepts percentage strings up
to 1024 characters with at most six fractional digits, beyond fixed-width integers.

## Decision

Use `num-bigint` and `num-rational` inside the pure domain calculator. The
maintainer approved arbitrary precision rather than a new numeric overflow limit.
`NominalAnnualRate` explicitly identifies a nominal annual percentage and matches
the configuration grammar, including zero and rejection of negative rates.

`daily_interest` takes a closing `Money` base, an explicit rate and a
`BusinessDate`. It returns `DailyInterest`, a nonnegative reduced rational in
minor units with an explicit currency and positive denominator. It uses the
supplied date's Gregorian year to select 365 or 366, and floors a negative base
to zero for calculation only. No inputs are mutated.

## Consequences

- Arithmetic is exact across the accepted rate range without floating point or
  daily rounding. Two small Rust numeric dependencies replace a custom bigint.
- Input length bounds parsing and arithmetic cost; this does not promise unlimited
  memory or infallible allocation.
- Numerator/denominator access exposes `BigInt` in this new Rust API.
- No payout rounding, period accumulation, rate-history selection, capitalization,
  database access, clock access, posting or HTTP endpoint is included.
- Callers must select the applicable effective-dated rate and provide the closing
  base; a future forecast layer is responsible for projected capitalization.
