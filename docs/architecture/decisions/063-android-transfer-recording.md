# ADR-063: Reviewed transfers through the existing recording writer

- **Status:** Accepted; implemented and verified locally
- **Date:** 2026-11-09
- **Related:** [ADR-060](060-android-income-expense-recording.md), [transfer policy](../../product/currencies-and-transfers.md), [execution plan](../../plans/active/2026-11-09-android-transfers.md)

## Context
P11.07 needs an Android transfer form and explicit confirmation. Shared transfer
commands, HTTP serialization and financial validation already exist. The current
signed-in recording coordinator owns the encrypted journal and authenticated
writer, but permits only income/expense.

## Decision
Extend that coordinator and modal rather than create a second writer. Existing
Home, Transactions and account-detail entry points expose a Transfer type;
account details retain their source-account preselection. Currency comes from
each selected active account. Both actual amounts are entered explicitly and
must match for same-currency transfers. Savings can send or receive; no balance
check is introduced. Transfers never have an income/expense category.

Review shows both account names (safe fallback after recovery), currencies,
actual amounts, business date, profile zone and posting/report effects. An
optional directional quote produces an exact half-even indicative received
amount, signed difference and informational discrepancy. It never changes actual
amounts. Common decimal digit multiplication avoids platform-specific BigDecimal,
binary floats and intermediate Long overflow. Reject an out-of-range indicative
amount before a new write. There is no rate lookup or automatic conversion.

Keep the existing v1 income/expense journal encoding byte-for-byte. New transfers
use a strict v2 payload with destination, received amount/currency and nullable
quote. The encryption wrapper, key alias, storage location and owner/origin
binding do not change. Old unknown versions still fail closed; no destructive
migration or rewrite occurs. An older app cannot resolve a v2 record and must
fail closed instead of replacing it.

Recovery reconstitutes the immutable command and operation key before loading
current account choices; no automatic POST occurs. Explicit retries use the
existing authenticated backend and server idempotency, even after account
eligibility changes. Disposing the signed-in flow still retires its presentation
and cannot acknowledge or clear an in-flight operation after retirement.

## Alternatives and consequences
A separate transfer writer would need another journal or coordinate concurrent
writers over the same unresolved-operation slot. Reuse avoids that race, but
requires regression tests for income/expense and legacy journal compatibility.
A platform decimal dependency would simplify arithmetic but add differing target
behavior; bounded digit multiplication keeps the comparison common and exact.
Backend posting, audit, ownership and permanent-key authority remain unchanged.
