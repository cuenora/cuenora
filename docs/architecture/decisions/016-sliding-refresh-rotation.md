# ADR-016: Rotate refresh credentials with a sliding session lifetime

- Status: Accepted; persistence and public refresh HTTP implemented
- Date: 2026-10-27
- Related: [ADR-005](005-identity-storage.md), [ADR-012](012-login-credential-primitives.md), [ADR-013](013-access-token-storage.md)

## Decision

Each successful refresh consumes one opaque credential and issues an independent
access/refresh pair within the same session family. Access remains valid for
15 minutes; the successor refresh expires 30 days after rotation. There is no
absolute session-age cap. Only successful refresh extends the deadline; ordinary
API activity alone does not. This maintainer-approved convenience policy is
temporary direction until separately designed passkey authentication exists.

Presenting any consumed refresh generation, including an expired one, revokes
its whole family and returns the same authentication rejection as an invalid
credential. Other sessions, even for the same user, remain unaffected. Already
revoked families, unknown/malformed secrets and expired unconsumed generations
cannot rotate and do not mutate stored state. An expired token is rejected at
the database eligibility check, including equality with its deadline.

## Transaction and ownership boundary

The adapter accepts a complete `cr1_` credential with exactly 64 lowercase hex
digits. It hashes the entire value with SHA-256 and derives the user/family from
the stored digest; callers cannot provide an owner or session ID. Generation
reuses the existing cryptographic randomness and token-lifetime constants.

Rotation and revocation lock the same owning session row. An explicit READ
COMMITTED transaction reads consumption, revocation and current expiry in a
fresh statement after acquiring that lock, rather than reusing a waiting
statement's old snapshot/time. The successor is inserted before the predecessor
link to satisfy the immediate ownership foreign key. Successor creation, access
creation and predecessor consumption share one database instant and transaction.
Any failure rolls back all those writes; replay rejection instead commits the
family revocation. SQLx/storage/entropy errors expose only a fieldless safe error.

`session_store::revoke` uses the same lock and accepts possession of any retained
refresh generation, including expired/consumed ones. Revocation is idempotent and
does not reset its timestamp. This internal persistence boundary is not a logout
or global-revocation HTTP endpoint. Existing rows, schema and dependencies remain
unchanged. Consumed history is retained for replay detection.

## Consequences and limits

- Two simultaneous refreshes have at most one issuance; the loser detects reuse
  and revokes the family, making even the winner unusable for later refresh.
- Clients must serialize refresh and replace stored secrets together. A lost
  successful response followed by retrying its predecessor forces fresh login;
  there is no retry grace period or network-level exactly-once guarantee.
- Old access rows are retained. Rotation alone does not revoke them; access
  validation must check their expiry and owning family's revocation. Bearer
  middleware remains P04.05, so persistence alone grants no HTTP authority.
- Sliding expiration permits a stolen current credential to extend a session
  until expiry, revocation or detected reuse. Cuenora's lack of money-transfer
  capability does not remove the sensitivity of financial records.
- The replay response follows the design guidance in
  [RFC 9700 section 4.14.2](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.14.2),
  without claiming that Cuenora implements an OAuth authorization server.

The public boundary is defined in [ADR-017](017-refresh-http.md); it reuses this
adapter without changing the lifecycle or introducing logout/global endpoints.
