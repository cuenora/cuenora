# ADR-051: Exact JSON amount adapters

- **Status:** Accepted
- **Date:** 2026-11-07
- **Scope:** P10.05

## Decision

Use kotlinx.serialization JSON 1.11.0 in the shared adapter package
`org.cuenora.shared.serialization`. Keep ADR-050 domain classes unchanged, without
serialization annotations. Handwritten `KSerializer` implementations need no
compiler plugin. Expose the runtime as an API dependency because serializer types
appear in public Kotlin signatures. Native frameworks do not re-export the JSON
library as a Swift API; native host convenience APIs remain future integration.

`MoneyAmountSerializer(currency)` handles an amount field, not an entire object.
The containing resource supplies the explicit currency. Encoding a different
currency fails rather than silently relabeling it. Decode the Rust recorded-money
grammar `-?[0-9]+(\.[0-9]{1,2})?`, including leading zeros and signed zero; encode
canonical signed amounts with exactly two fractional digits. Overflow, including
Long.MIN_VALUE, fails without rounding. Parsing uses ASCII digits and integer text.

`ProjectedAmountSerializer` maps arbitrary-precision nonnegative
`ProjectedMinorUnits` to/from two-decimal payout strings without Long conversion.
Use `.nullable` for an open tail. `ReportAmountSerializer` validates signed,
arbitrary-precision two-decimal strings and preserves their text, matching
OpenAPI `ReportAmount`. Neither serializer converts projections/totals to Money.
Rates and exact rational accruals have different contracts and are not money.

All JSON adapters reject non-string tokens, including numeric primitives in tree
or lenient decoding. Invalid-amount diagnostics do not interpolate input values;
callers must still avoid logging JSON parser exceptions or financial payloads.

## Alternatives and tradeoffs

A pure text codec would defer JSON proof, contrary to the maintainer-approved
scope. Domain annotations would couple domain modeling to transport. Complete
resource DTOs and generated serializers would expand this task into P10.06.
Explicit serializer arguments preserve domain independence at the cost of wiring
currency from the containing resource in future DTO mapping.

## Verification

Shared tests use actual JSON text/tree encoding and decoding on Android host and
iOS ARM64 simulator. Test symmetric bounds, values beyond binary floating-point
integer precision, malformed inputs, currency mismatch, arbitrary-precision
projections/totals, nullable projections, and numeric-token rejection.
Rebuild all shared artifacts because the new dependency affects both platforms.
