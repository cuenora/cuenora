# ADR-030: Independent permanent expense retries

- **Status:** Accepted
- **Date:** 2026-10-30
- **Roadmap:** P05.10

## Decision

Accountless expenses use mandatory `Idempotency-Key` and permanent owner-scoped
retry records in `expense_creation_keys`, independent of income keys. The
maintainer approved this policy. The same key can create one income and one
expense; changing an expense request with its existing key conflicts.

Follow ADR-029's normalized, length-framed hash, owner-row serialization,
verified-owner replay and atomic event/audit/key insertion. Expense fingerprints
use `accountless-expense-v1`; no income fingerprint or stored key changes.
Retries reconstruct the original response without storing financial snapshots.
Expense keys reject update, delete and truncate and have no TTL.

## Consequences and alternatives

Dedicated storage costs one small permanent row per expense and another table,
but avoids cross-kind conflicts and changing existing income retry semantics.
A shared key table would require a compatibility migration and coupled namespaces;
it is not needed for P05.10. Linked expenses remain P05.11; editing, voiding and
reports remain outside this change. No account movement is created.
