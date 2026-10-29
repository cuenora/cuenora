# ADR-023: Uniform API versioning and lifetime account-creation idempotency

- **Status:** Accepted; implemented locally with own and independent checks
- **Date:** 2026-10-29
- **Roadmap:** P05.02

## Decision

Move every implemented endpoint, including authentication and health, under `/v1`
without unversioned aliases. The maintainer explicitly chose a uniform breaking
change during development instead of mixed versioned/unversioned routes.
This supersedes the path compatibility assumptions in ADR-011, ADR-015,
ADR-017, ADR-018 and ADR-021; their authentication and security policies remain.
Future verification/recovery routes in ADR-022 use the same prefix.

Require `Idempotency-Key` for `POST /v1/accounts`. Scope keys to the authenticated
owner and this operation. Keep each key and original creation result for the
account lifetime, with no expiry. Same normalized input returns the original
201 result; different normalized input returns 409 with no mutation. Both key
requirement/replay semantics and lifetime retention were explicitly approved.

Transport keys are case-sensitive, 1–128 ASCII letters, digits, hyphens or
underscores; clients should generate a fresh random key per intended creation.
Persist only its SHA-256 digest. Compare the validated name, type and currency
rather than hashing ambiguous serialized JSON. Field order/JSON whitespace and
outer name whitespace are immaterial; remaining name text is exact.

Use the existing user row as a transaction lock: recheck verified email, look up
the owner's key, insert account and key atomically, then commit. Serialize only
creations for that owner, with bounded lock/statement timeouts. A lost commit
response is an uncertain outcome: retry the same key, never create a fresh one.

## Tradeoffs and boundaries

Uniform versioning breaks old paths and requires updating probes and consumers
at once, but avoids compatibility machinery before external users. Lifetime
retention consumes one record per account and retains the original name; it
prevents late retries from silently duplicating accounts. Per-owner serialization
limits concurrent creation throughput but avoids speculative inserts and complex
advisory-lock mapping. This is not a generic financial-posting idempotency policy.

Ownership comes only from bearer identity; verification is checked even on
replay. Normal/savings types, supported currencies and names reuse the domain.
No opening balances, zone-dependent dates, posted movements or client owner fields
are accepted. Verification delivery/confirmation remains unimplemented; seeded
verified identities are test fixtures, not an end-to-end product journey.
