# ADR-029: Permanent accountless-income retry equivalence

- **Status:** Accepted
- **Date:** 2026-10-30
- **Roadmap:** P05.08

## Context

An accountless income contributes to income reporting without any account
movement. A lost HTTP response must not require risking a duplicate income.
The maintainer approved mandatory `Idempotency-Key`, following account creation.
Existing transaction storage and audit do not supply retry equivalence.

## Decision

Implement `POST /v1/transactions` for accountless ordinary income only. Require
verified bearer ownership, a confirmed IANA profile zone and an explicit valid
past/today business date. Reuse exact Money and BusinessDate validation. Preserve
optional income category and description fields from the storage model.

Retain `(user_id, key_hash)` permanently in a dedicated `income_creation_keys`
table with an owned transaction reference and a hash of the normalized request.
Frame fields with presence and byte lengths before SHA-256 hashing. No financial
value snapshots or raw keys are stored there. Null/absence and exact Money
formatting normalize; description bytes remain significant.

Lock the owner row before reading retry state, following account creation and
profile updates. Atomically persist one income event, empty-field-name creation
audit and retry record; never insert a movement. Equivalent requests reconstruct
the original response from the equivalent normalized input and stored ID rather
than reading potentially edited event fields. Verification remains required on
replays, but a changed zone does not invalidate an accepted historical date.
Retry records reject UPDATE, DELETE and TRUNCATE; they have no TTL.

## Consequences and alternatives

- Permanent storage costs one small row per operation, but avoids TTL-induced
  duplicates. Key rotation by a caller explicitly means a new operation.
- Owner-level serialization is simpler and consistent with existing workflows,
  at the cost of serializing simultaneous financial writes by one owner. Lock
  and statement timeouts bound waits; callers retry the same key on uncertainty.
- Storing full response snapshots would simplify replay, but would duplicate
  sensitive description/financial values. Reconstruction avoids that duplication.
- The endpoint cannot link accounts or create expenses yet. Those remain P05.09–11;
  future operations must explicitly define their key namespaces and compatibility.
- No email verification bypass, usable verification journey or reporting endpoint
  is introduced. Public contract: [accountless income](../../product/accountless-income.md).
