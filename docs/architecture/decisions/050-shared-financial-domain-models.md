# ADR-050: Transport-independent shared financial models

- **Status:** Accepted
- **Date:** 2026-11-07
- **Scope:** P10.04

## Decision

Keep Kotlin models in `org.cuenora.shared.domain`, without HTTP, JSON, database,
clock or platform dependencies. The maintainer explicitly chose domain models
rather than API response replicas. P10.05 owns decimal serialization and P10.06
owns the HTTP client; neither is introduced here.

`Money` stores signed `Long` hundredths and an explicit SEK/EUR/USD currency,
excluding `Long.MIN_VALUE` to match Rust's symmetric range. It provides no implicit
FX, arithmetic or formatting. `BusinessDate` stores validated Gregorian components
in years 1–9999, not a timestamp; callers supply dates without a hidden clock.
Opaque typed identities carry two 64-bit UUID halves; generation/parsing belongs
outside the domain. They are not credentials or authorization evidence.

`Account` contains metadata and owner identity, not balances or interest terms.
`AccountName` matches Rust's Unicode whitespace and 255-scalar policy, rejects
invalid surrogate sequences, and preserves internal whitespace/case/normalization.
Do not substitute Kotlin's broader `isWhitespace` behavior for Rust `str::trim`.

`Transaction` is one recorded event, including accountless income/expenses and
voided history. Transfers retain two distinct account identities and positive
sent/received `Money` values; same-currency amounts match. Opening amounts may
be signed. Interest may retain its income category. No model calculates balances,
posts movements or establishes ownership, savings-account eligibility, category
catalog membership, or whether a business date is allowed for a command.
Those decisions require application/server context.

`SavingsForecast` follows the pure Rust output: a currency and inclusive periods.
Exact accrual uses nonnegative integer numerator/positive denominator text;
projected payouts use a distinct arbitrary-precision integer-minor-unit type,
nullable for an open tail. Text is canonical ASCII integer digits, never binary
floating point. These values are structural exact representations, not a rational
arithmetic/normalization engine. Period-close half-even rounding is the producer's
responsibility. Projected values cannot be passed as recorded `Money`. Lists are
defensively copied; API snapshot flags/rate-selection metadata stay outside this
primitive. There is no client-side forecast calculator.

## Tradeoffs and alternatives

Explicit date components and integer digit values avoid a new datetime/big-number
runtime dependency for a modeling-only step. They do not provide parsing,
arithmetic or locale formatting; introduce those deliberately when needed, with
cross-platform tests. API-shaped DTOs would simplify mapping initially but couple
the domain to transport names and snapshot metadata. Mapping and any quoted-rate
presentation details belong to later adapter/use-case work, not this domain core.

## Verification

Shared constructor/value tests run on Android JVM host and iOS ARM64 simulator.
Rebuild all three shared artifacts to verify Kotlin/Native export compatibility.
The existing shared marker test remains a regression check for P10.03.
