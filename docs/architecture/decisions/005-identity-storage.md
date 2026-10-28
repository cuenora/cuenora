# ADR-005: Minimal identity and opaque session storage

- **Status at decision:** Accepted; schema implemented, authentication runtime pending
- **Date:** 2026-10-24
- **Scope:** P03.04; extends [ADR-004](004-first-party-identity.md)


**Implementation update (2026-10-28):** Authentication/session runtime and
[route-scoped identity](018-authenticated-identity.md) now exist. Verification/recovery
and financial resource authorization remain pending. The decision and original
scope below are retained; see the [current ADR index](README.md) and
[master-plan snapshot](../../plans/master-plan.md#current-delivery-snapshot).

## Decision

Use PostgreSQL-generated UUIDs as stable user identifiers. Separate `users`,
`password_credentials`, `user_sessions` and `session_refresh_tokens`; financial
ownership will reference user IDs. A missing profile zone or email verification
timestamp represents an incomplete registration, without granting access.

The maintainer approved case-insensitive email login and an MVP restricted to
ASCII local parts with IDNA domains. Preserve original input and a separately
validated ASCII transport address; derive the unique lowercase comparison key
in PostgreSQL. Do not remove dots or plus tags. This is Cuenora's identifier
policy, not a claim about every SMTP mailbox's case sensitivity.

Use opaque, server-tracked refresh sessions rather than self-contained refresh
JWTs. Store only 32-byte token digests and retain consumed generations with
same-session replacement links. A stable session row supports later family
revocation. Expiry, ownership and paired rotation markers have database guards;
generation, secure digest calculation, atomic rotation, locking, reuse detection
and authorization remain Phase 04 application responsibilities.

## Alternatives and tradeoffs

| Alternative | Tradeoff behind the choice |
| --- | --- |
| Unicode local parts | Broader mailbox support, but Resend SMTPUTF8 support is unconfirmed. The maintainer accepted the conservative MVP scope. |
| Caller-provided comparison key | Simpler schema, but could disagree with the stored address. A generated key enforces one ASCII case policy. |
| Self-contained refresh JWTs | Can carry claims, but revocation/reuse still needs server state. Opaque tokens keep this storage boundary direct. Access-token format remains undecided. |
| One row overwritten on refresh | Smaller state, but loses consumed generations needed for later reuse detection. |

## Migration and security boundary

Versioned SQL files contain forward DDL without transaction control. The current
isolated test runner applies each file with `psql --single-transaction` and
`ON_ERROR_STOP`; a future SQLx runner can own version tracking and transactions.
Do not introduce an ad hoc migration ledger or hide drift with `IF NOT EXISTS`.
No SQLx dependency or application database connection is added by this decision.

The schema checks storage shape, not complete email/IDNA, IANA-zone or Argon2 PHC
validity. Authenticated ownership, verification gates, password hashing, challenge
tables, email changes and session protocols require explicit later implementation.
See [storage details](../identity-storage.md) for constraints and application duties.
