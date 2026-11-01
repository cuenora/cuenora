# ADR-042: Effective-dated savings configuration streams

- **Status:** Accepted; locally verified
- **Date:** 2026-11-01
- **Roadmap:** P08.02

## Context

Optional interest terms and rates need history without changing posted money.
The maintainer approved effective-dated payout/capitalization history and rates
with at most six percentage fractional digits, rejecting excess precision.

## Decision

Use two additive, append-only per-account streams: `savings_terms` and
`savings_rate_periods`. Each has one unique slot per inclusive effective date;
the next slot ends the prior version implicitly. Composite account/owner/type
references enforce savings-only ownership using the existing unique key.

Store annual percentages as exact unconstrained `NUMERIC` with explicit finite,
nonnegative and scale checks. Do not use a six-place typmod that silently rounds
inputs. Keep fixed calculation policies explicit in each terms row and require
an explicit capitalization choice. Add no default configuration or postings.

## Consequences

Independent streams avoid copying rates when cadence changes; a future forecast
must establish complete coverage from both streams. Immutability preserves prior
conditions but means correction workflows require a separate design. No custom
payout calendar, calculator, configuration API or automatic financial effect is
introduced. See [storage details](../savings-storage.md).
