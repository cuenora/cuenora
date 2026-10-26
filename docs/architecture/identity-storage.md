# User identity and session storage

P03.04 adds versioned migrations in `backend/migrations/` and isolated
PostgreSQL acceptance tests. P04.02 subsequently added registration and its
PostgreSQL adapter. P04.03 adds access-token digest storage and atomic initial
session issuance; HTTP login, email delivery and financial authorization remain
pending.
[ADR-005](decisions/005-identity-storage.md) records the storage choices.

## Tables and ownership

| Table | Stored state and guards |
| --- | --- |
| `users` | UUID primary key; original and ASCII transport email; generated unique lowercase email key; nullable verification instant and profile zone; UTC audit instants. |
| `password_credentials` | At most one encoded Argon2id credential per user; nonempty bounded hash representation and password-change instant. |
| `user_sessions` | UUID session/family identifier, user ownership, creation and nullable revocation instant. |
| `session_refresh_tokens` | Unique 32-byte digest, owning session/user, creation/expiry, nullable consumption instant and replacement ID. |
| `session_access_tokens` | Unique 32-byte digest, owning session/user, creation and expiry; independent of refresh rotation. |

UUID defaults are server generated. Required owner references reject missing users;
composite foreign keys prevent a token from claiming another session's owner or
linking to a successor in a different session, even for the same user. Referenced
rows use restrictive deletion rather than cascading away identity history.

All audit instants use `TIMESTAMPTZ(6)`, bounded to years 0001–9999; infinity is
rejected. Creation defaults use the statement instant. PostgreSQL represents
instants independently of display time zone; applications must serialize them in
UTC. Updates must maintain `updated_at` explicitly; no update trigger is implied.
The profile zone remains NULL until confirmation. Its database guard only checks
nonempty bounded storage: application code must validate an actual IANA identifier.

## Email persistence contract

The future application must validate one mailbox, accept only an ASCII local
part, convert international domains through IDNA, and lowercase the transport
domain before persistence. Store the original input separately. For example:

| Column | Example |
| --- | --- |
| `email_original` | `AnA@mañana.example` |
| `email_address` | `AnA@xn--maana-pta.example` |
| `email_key` (generated) | `ana@xn--maana-pta.example` |

Case variants identify the same user; dots and plus tags remain significant.
The generated key uses PostgreSQL's `C` collation for deterministic ASCII casing.
Callers cannot provide their own key. The transport address is ASCII and bounded
to 254 octets, with a 1–64 character local part; original input is bounded to
1024 octets and must preserve that same local part. These are storage guards,
not an RFC mailbox parser or proof that the original and IDNA domains correspond.

Resend is the planned provider. Its SMTPUTF8 support was not confirmed from public
documentation; the maintainer accepted this MVP scope while that remains unknown.
No provider account was accessed and no email integration or delivery is present.

## Session persistence contract

Refresh tokens are opaque random secrets; this schema stores only their
32-byte digests, never raw secrets. The length check cannot prove entropy or
cryptographic hashing. P04.03 credential primitives now generate independent 256-bit secrets with
SHA-256 digests and approved access/refresh lifetimes (15 minutes/30 days); see
[ADR-012](decisions/012-login-credential-primitives.md). Initial persistence is
implemented by the login-store adapter; HTTP login remains pending.

Every token expires after its creation instant. A consumed token pairs `used_at`
with `replaced_by`; neither marker can be set alone. Replacement must name another
token in the same session. Consumed rows remain available after rotation and
family revocation. The database does not implement one-time consumption, prevent
all graph cycles, enforce event ordering or serialize concurrent rotations.
Those protocol guarantees require transactional application logic and tests in
P04.04. Revocation state alone does not reject an HTTP request.

Access-token rows similarly store only a unique 32-byte digest and bind it to the
same user-owned session. The schema checks digest length, expiry ordering, finite
timestamps and restrictive ownership; length does not prove SHA-256 hashing or
token entropy. Application code owns cryptographic token generation, hashing,
the approved 15-minute lifetime, digest lookup, expiry checks and checking the
owning session's `revoked_at`. Initial session issuance rechecks the credential
hash and password-change revision under a row lock and atomically inserts one
session, access digest and refresh digest. There are no access-token rotation or
per-token revocation fields. Issuance alone does not authenticate bearer
requests; HTTP lookup and revocation checks remain pending. See
[ADR-013](decisions/013-access-token-storage.md) and
[ADR-014](decisions/014-initial-session-issuance.md).

Password storage accepts a bounded, control-free `$argon2id$` representation;
P04.02 implements hashing, salts and input policy. P04.03 primitives add bounded
PHC parsing and exact-byte verification; public login/session integration remains
pending.
Verification and recovery challenges need a later focused migration. Any future
email-change operation must reset verification atomically with the address change.

## Apply, verify and recover

Run from the repository root with local Docker Compose v2 available:

```bash
python3 tests/infrastructure/test_postgres_compose.py
```

The runner creates unique disposable projects with random credentials, applies
SQL files in filename order with `psql -X --single-transaction -v ON_ERROR_STOP=1`,
and removes its own containers, networks and volumes. It ignores developer `.env`
files. Never direct it at a personal or production database. Reapplying a file
fails without changing state; the harness has no migration ledger or resume mode.

A failed file rolls back that file's DDL; previously committed files remain.
For these disposable tests, recreate the project to retry a clean apply. There
is no production deployment/down-migration procedure yet. Once application data
exists, use reviewed forward corrections and backup/restore plans rather than
dropping identity tables. The development guide records [complete-schema verification](../development/SETUP.md#fresh-migration-verification-p0311)
and [SQL constraint/owned-relationship coverage](../development/SETUP.md#sql-constraints-and-owned-relationships-p0312).
