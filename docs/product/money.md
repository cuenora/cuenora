# Exact money: representation and acceptance cases

**Roadmap:** P03.01–P03.02 · **Status:** Implemented and locally verified · **Date:** 2026-10-24

Represent recorded money as signed integer hundredths plus an explicit currency.
Keep fractional calculation results separate until an explicit rounding step.
[ADR-002](../architecture/decisions/002-exact-money.md) records the tradeoffs.

## Approved product policies

The maintainer approved these policies individually:

- Recorded SEK, EUR and USD amounts accept at most two decimal places. Reject
  extra decimal places instead of silently rounding an amount from a bank record.
- Recording an expense or transfer may leave a negative account balance.
  Cuenora records movements; it does not authorize or execute bank payments.
- Calculated results use round-to-nearest, ties-to-even by default. Specific
  account interest terms take precedence; actual credited amounts are recorded exactly.
- No commercial amount cap applies. Reject technical overflow without truncation
  or any change to balances.

## Engineering contract

| Concern | Decision |
| --- | --- |
| Currency | Closed `Currency` type: `SEK`, `EUR`, `USD`; uppercase codes only. |
| Recorded value | `Money` contains private signed `i64` minor units and a currency. One unit is 0.01 of that currency. |
| Valid minor units | `-9223372036854775807` through `9223372036854775807`, inclusive. |
| Valid amounts | `-92233720368547758.07` through `92233720368547758.07`, inclusive. |
| Symmetry | Exclude `i64::MIN`, so every valid amount has a valid exact negation. |
| Zero | Valid for balances and arithmetic; normalize negative zero to zero. |
| Business entry amounts | Income, expense and transfer amounts must be strictly positive at their use-case boundary; signed movements and negative openings/balances remain representable. |
| Arithmetic | Checked same-currency addition, subtraction and negation; no implicit FX or cross-currency ordering. |
| Failure | Return typed errors for unsupported currency, invalid decimal syntax, excessive scale, invalid ratio, currency mismatch and out-of-range results. No panic, wrapping or saturation for invalid input. |
| Formatting | Canonical decimal string with a dot, exactly two fractional digits, no grouping or leading plus; retain the currency separately. |

The symmetric range is an engineering bound, not a per-transaction bank limit.
Validate it at every constructor and after every arithmetic or rounded result.
Checking machine overflow alone is insufficient: an operation may produce the
excluded `i64::MIN` without overflowing the machine type.

### Recorded input and output

The domain decimal parser accepts ASCII `-?[0-9]+(\.[0-9]{1,2})?` only, with no
surrounding whitespace. Leading zeros are accepted and normalized. Reject a third
fractional digit even when it is zero: `10.500` is invalid recorded input.
No constructor accepts a binary float.

Examples in the Spanish discussion used a comma as a human decimal separator.
Domain and future API strings use a dot. A future localized UI may normalize its
explicit locale format before domain validation; comma/grouping parsing is not
part of this Rust parser. Monetary API values remain JSON decimal strings, never
JSON numbers. No financial HTTP endpoint is implemented by these domain types.

`Money` itself accepts signed values. Positive-entry validation belongs to future
use cases, not to the generic value object. Zero income/expense/transfer rejection
and negative-balance posting are specified here but become use-case tests in
P05/P06; P03 tests only the underlying value semantics.

### Calculated results and rounding

Provide an explicit, checked construction path from an exact rational number of
minor units: signed `i128` numerator, strictly positive `i128` denominator and
currency. This is a quantization primitive, not an interest or FX engine. For
example, `10005 / 10` minor units represents `10.005` major units before rounding.

Keep quotient and remainder exact. Choose the nearest integer minor unit; at an
exact half, choose the even integer, symmetrically for negative values. Check the
rounded result against the `Money` range. Avoid intermediate overflow, including
absolute value of `i128::MIN` and doubling a large remainder; invalid input must
return an error. Do not round each intermediate term in a future calculation:
its settlement boundary and precision must be specified by that calculation.

The rational input range is a technical limit of this primitive, not a prescribed
precision for future rates or interest engines. Those engines must use exact,
checked representations and adapt to this boundary without hidden rounding, or
explicitly extend it. Do not introduce a decimal library just for P03 addition
and subtraction. No `Money` multiplication, division, rate model or automatic
currency conversion is required by this work unit.

## Acceptance cases for implementation

These cases are covered by [public-interface Rust tests](../../backend/crates/domain/tests/money.rs),
observed failing before implementation and passing afterward. `M` means
`92233720368547758.07` in the currency under test. Run single-currency cases for
SEK, EUR and USD. Check typed errors and unchanged operands on every failure.

| ID | Input or operation | Expected result |
| --- | --- | --- |
| C01 | Currency `SEK`, `EUR`, `USD`; then `GBP`, `sek`, empty | Supported codes accepted; remaining codes rejected. |
| M01 | Parse `10`, `10.0`, `10.00`; then `10.5`, `10.50` | First group: equal at 1000 minor units, format `10.00`. Second group: equal at 1050 minor units, format `10.50`. |
| M02 | Parse `0010.50`, `0`, `-0.00`, `-0.01` | Format as `10.50`, `0.00`, `0.00`, `-0.01`. |
| M03 | Parse `10.505`, `10.500` | Excessive-scale error; never round either input. |
| M04 | Parse empty, ` 10`, `10 `, `+10`, `10,50`, `1e2`, `.50`, `10.`, `NaN`, `∞` | Invalid-decimal error. |
| M05 | Parse `M`, `-M` and construct their minor-unit values | Both endpoints accepted and formatted exactly. |
| M06 | Parse `92233720368547758.08`, `-92233720368547758.08` and an integer beyond machine range | Out-of-range error. |
| M07 | Construct from `i64::MIN` | Out-of-range error, despite being a valid machine integer. |
| A01 | `0.10 + 0.20`; `0.30 - 0.10` | Exactly `0.30`; `0.20`. |
| A02 | `100.00 - 150.00`; `-50.00 + 50.00` | `-50.00`; `0.00`. No insufficient-balance error in value arithmetic. |
| A03 | Add/subtract SEK and EUR, including equal numeric values and zero | Currency-mismatch error; no combined total or conversion. Repeat for each distinct currency pair. |
| A04 | `M + 0.01`; `-M - 0.01`; `M - (-0.01)` | Out-of-range error, including results equal to excluded `i64::MIN`. |
| A05 | Negate `M`, `-M`, zero; negate twice | `-M`, `M`, zero; original value restored. |
| A06 | `M + 0.00`; `M - M`; `-M + M` | `M`; zero; zero. |
| R01 | Quantize minor-unit ratios `10004/10`, `10005/10`, `10006/10`, `10015/10` | `10.00`, `10.00`, `10.01`, `10.02`. |
| R02 | Quantize `-10005/10`, `-10015/10`, `5/10`, `15/10`, `-5/10` | `-10.00`, `-10.02`, `0.00`, `0.02`, canonical `0.00`. |
| R03 | Quantize `92233720368547758074/10`, `92233720368547758075/10` | `M`; out-of-range error after rounding. Repeat with negative numerators. |
| R04 | Denominator zero or negative | Invalid-ratio error; no division panic. |
| R05 | Numerator `i128::MIN`, denominator 1; numerator 1, denominator `i128::MAX` | Out-of-range error; zero without intermediate overflow. |
| R06 | Numerator `i128::MAX - 1`, denominator `i128::MAX` | `0.01`, without overflow while comparing a remainder to half the denominator. |

For every accepted value, parsing its canonical formatted string must reproduce
both its minor units and currency. Derived ordering, if
offered, must not invent an ordering across currencies.

## Implementation and verification boundary

The public domain interface is implemented in `cuenora-domain` with no HTTP,
database, clock or UI dependencies. The exported types are `Currency`, `Money`
and `MoneyError`. Use `Money::parse(amount, currency)` for recorded decimal
strings, `Money::from_minor_units` for exact hundredths and
`Money::from_minor_ratio` for explicit half-even quantization. `minor_units()`
and `currency()` expose the value; `Display` formats the amount only.
`checked_add`, `checked_sub` and `checked_neg` return `Result<Money, MoneyError>`.
Currency implements `FromStr` and `Display`; errors implement the standard
`Error` trait without retaining input values. There is no cross-currency ordering.

All 20 deterministic test groups passed after an observed RED/GREEN cycle.
The existing API liveness test also passed. There are no domain commands or options
requiring a CLI differential check. Run from `backend/`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

All three checks passed using the pinned 1.99.1 compiler; the
[execution plan](../plans/completed/2026-10-24-money-specification.md) records commands
and the initial failing runs. P03.03 still owns date policies; P06.01 owns rate precision and
discrepancies; P08.01 owns specific interest terms and settlement boundaries.
Persistence column types and constraints require their own migration decisions.

## References

- [Approved currencies and transfer boundary](currencies-and-transfers.md).
- [SIX ISO 4217 current list](https://www.six-group.com/dam/download/financial-information/data-center/iso-currrency/lists/list-one.xml), published 2026-09-17: SEK, EUR and USD use two minor-unit digits.
- [Rust signed 64-bit integers and checked arithmetic](https://doc.rust-lang.org/std/primitive.i64.html).
- [Execution plan](../plans/completed/2026-10-24-money-specification.md).
