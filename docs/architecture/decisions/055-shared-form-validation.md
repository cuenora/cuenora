# ADR-055: Advisory shared form validation

- **Status:** Accepted
- **Date:** 2026-11-07
- **Related:** P10.09, ADR-050, ADR-051, ADR-054

## Context

Native forms need reusable feedback without UI or server orchestration. The
maintainer included authentication, recurrence and savings configuration alongside
accounts and transactions, accepting server-only IDNA, common-password and
supported-zone checks without new dependencies.

## Decision

Add opt-in common Kotlin validation functions with field/code issues and
`isLocallyValid`. `serverChecksRequired` always remains true. Results never copy
submitted values; callers localize codes and must not log sensitive inputs.
Input containers are ordinary data, not redacted logging wrappers.

Reuse AccountName, BusinessDate, MoneyAmountSerializer and Transaction invariants.
Validate complete merged financial edit forms; callers retain PATCH presence and
clear semantics. Metadata enables advisory account checks only. Opening balances
belong to account creation. Exact rates remain strings; no FX/interest calculation.

Preserve password bytes. Signup/reset check 15–128 Unicode scalars; login checks
only existing upper bounds. Email structure checking leaves IDNA/canonicalization
to the server. Temporary mapping of IDNA separators never rewrites submitted
email. Local success neither authenticates nor verifies a token or ownership.

## Consequences

Hosts share deterministic feedback, not UI state. They must handle server
rejection including common passwords, unsupported zones, IDNA constraints, quote
numeric overflow, dated posting policy, conflicting configuration and stale
metadata. No dependencies, backend changes, schedule engine or auth orchestration.
