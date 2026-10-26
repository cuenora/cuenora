# ADR-014: Issue initial sessions from current credential snapshots

- Status: Accepted; credential lookup, issuance and HTTP login implemented
- Date: 2026-10-26
- Related: [ADR-012](012-login-credential-primitives.md), [ADR-013](013-access-token-storage.md)

## Context

Password verification must not authorize a session if the account credential
changes between verification and persistence. Initial issuance also needs to
create one session, one access digest and one refresh digest as a single
database operation without persisting bearer secrets.

## Decision

Credential lookup returns an opaque snapshot binding the server-owned user ID,
encoded password hash and exact `password_changed_at` microsecond revision.
The snapshot provides the encoded hash for password verification and has no
formatting, serialization or caller-settable owner/revision interface. Lookup
alone is not authentication: the caller must verify the supplied password against
that hash before requesting issuance.

Issuance locks and rechecks both the stored hash and revision, then inserts the
session and both token rows in one parameterized statement. A stale snapshot
creates no rows; a storage or constraint failure returns a fieldless safe error
and the statement rolls back. Token rows contain only SHA-256 digests of the
complete purpose-prefixed tokens. PostgreSQL supplies row IDs and timestamps;
fixed 15-minute and 30-day lifetimes come from the shared token constants.

Lookup validates the address through the existing email policy and compares the
generated lowercase identity key with explicit `C` casing on the input, regardless
of the database default locale, preserving significant dots and plus tags.
It does not create, repair, link or rehash identities or credentials. Pending
users may receive sessions, but issuance grants no financial authority and
does not alter verification or profile state.

## Consequences and limits

- A password update serializes against issuance through the credential-row lock;
  a snapshot that loses the race cannot create a session.
- Each successful request creates a new independent session family. Refresh
  rotation/revocation and access-token validation remain separate work.
- A connection lost around commit can leave the caller uncertain; this operation
  does not claim network-level exactly-once delivery.
- Authentication middleware, refresh rotation/revocation and email behavior are not implemented.
