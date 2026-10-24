# ADR-002: Exact recorded money in signed minor units

- **Status:** Accepted; domain types implemented
- **Date:** 2026-10-24
- **Scope:** P03.01–P03.02; representation of recorded money and explicit quantization

## Context

The maintainer approved two-decimal SEK/EUR/USD recorded amounts without silent
rounding, negative balances, half-even calculated-result rounding by default and
technical-only monetary bounds. Domain code has no financial models yet.
[ADR-001](001-initial-stack.md) deliberately left monetary representation open.

## Decision

Use a currency-tagged `Money` value with private signed `i64` minor units, where
100 units equal one major currency unit. Use the symmetric range
`-9223372036854775807..=9223372036854775807`, excluding `i64::MIN` so that every
valid amount can be negated exactly for a reversal. All constructors and checked
arithmetic must enforce the domain range, including machine-representable results.

Parse and format decimal strings without floats. Keep signed money separate from
positive-only income/expense/transfer input validation. Reject cross-currency
arithmetic. Quantize exact calculated ratios explicitly using nearest/ties-even,
with a checked signed `i128` numerator and positive `i128` denominator; the ratio
is expressed in minor units. Specific account interest terms may override the rounding default.

The [money specification](../../product/money.md) defines bounds, syntax, error
behavior and acceptance cases. This refines the master plan's proposed decimal
arithmetic direction: recorded values use fixed-scale integers; fractional
calculation engines may later use an exact decimal or rational representation.
No library dependency, financial engine, database type or schema is introduced.
Financial API amounts remain decimal strings, regardless of internal storage.

## Alternatives and tradeoffs

| Alternative | Benefit | Tradeoff behind this decision |
| --- | --- | --- |
| Exact decimal library for all recorded values | Variable scale and decimal calculation helpers | More dependency and scale-normalization work than two-decimal addition/subtraction requires. Reconsider for future interest engines. |
| Signed minor units using the full `i64` range | One additional negative minor unit | The minimum has no representable positive counterpart, complicating reversals. |
| Arbitrary-precision arithmetic | Larger supported range | Adds complexity without an MVP requirement for amounts beyond the explicit technical bound. |
| Binary floats | Familiar arithmetic operators | Cannot represent all decimal fractions exactly; prohibited for monetary calculations. |

## Consequences and revisit conditions

Recorded arithmetic is exact and needs no new dependency. Fractional calculations
must retain precision outside `Money` and cross the explicit quantization boundary;
rounding a user-entered recorded amount is forbidden. The bound also applies to
Money-valued totals, not just individual entries; overflow must remain an error.

Future persistence must preserve this contract and must not silently round input.
Select database columns and constraints during the migration tasks. Revisit this
ADR if currencies with a different scale, amounts outside the range or calculation
requirements beyond the ratio boundary enter scope. Any later extension must
retain exact string interchange and compatibility with existing records.

## Implementation status

P03.01–P03.02 are implemented in `cuenora-domain` with no new dependencies.
Twenty public-interface test groups passed after observed tests-first RED and
GREEN runs; workspace tests, formatting and Clippy passed on Rust 1.99.1.
See the [execution record](../../plans/completed/2026-10-24-money-specification.md).
Financial application use cases and persistence remain pending.
