# ADR-013: Store opaque access-token digests per session

- Status: Accepted; storage and issuance implemented, bearer authentication pending
- Date: 2026-10-26
- Related: [ADR-005](005-identity-storage.md), [ADR-012](012-login-credential-primitives.md)

## Context

P04.03 issues opaque access and refresh credentials, but access credentials need
server-side persistence so later authentication can resolve their session and
reject expired or revoked sessions. The existing refresh-token table represents
rotation history and is not an access-token store.

## Decision

Add `session_access_tokens` as a forward-only migration. Each row stores a unique
32-byte token digest, its session and user IDs, and creation and expiry instants.
The composite foreign key binds the row to the matching user-owned session and
restricts deletion. Timestamp precision, finite ranges, statement-time defaults,
and session/user lookup indexes follow the existing identity schema.

Access tokens are independent rows, not refresh generations: this table has no
consumption, successor, or per-token revocation fields. Session revocation stays
on `user_sessions` and must be enforced by application validation.

## Responsibilities and limits

The database enforces digest uniqueness and length, but cannot prove the bytes
are a SHA-256 digest or that the original token was cryptographically random.
Application code must generate and hash opaque tokens, use the approved 15-minute
access lifetime, and resolve the digest while checking expiry and the owning
session's revocation state. The table does not authenticate bearer requests or
implement lookup, issuance, rotation, or revocation behavior. Initial issuance is
implemented in P04.03 and refresh rotation/revocation in P04.04; bearer lookup and
validation remain P04.05 application work.

## Consequences

- Access credentials can be looked up by unique digest without storing bearer
  secrets.
- Session/user ownership and restrictive deletion preserve identity history.
- Access validity depends on application checks; schema constraints alone do
  not authenticate a request or react to session revocation.
- Existing users, sessions, and refresh-token rows are unchanged by the
  migration.
