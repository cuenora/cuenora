# Initial identity and access strategy

**Roadmap:** P04.01–P04.12 and P03.04 · **Status:** Registration, login, rotation, identity middleware and logout implemented; email verification/recovery designed, runtime pending · **Updated:** 2026-10-28

Cuenora will initially authenticate users with its own email/password credentials.
Registration is open and self-service. Email ownership must be verified before
financial features become available. [ADR-004](../architecture/decisions/004-first-party-identity.md)
records the choice and its tradeoffs.

## Approved product decisions

| Decision | Initial behavior |
| --- | --- |
| Authentication owner | Cuenora manages email/password credentials rather than delegating sign-in to an OIDC provider. |
| Registration | Anyone may register; invitation or administrator approval is not required. |
| Verification gate | A new registration remains pending email verification. Financial account creation and recording movements are blocked until verification succeeds. |

The maintainer subsequently approved case-insensitive login identifiers and
ASCII local parts with IDNA internationalized domains. Preserve original input;
do not remove dots or plus tags. Resend is the planned mail provider, with
SMTPUTF8 support still unconfirmed and no integration implemented. Initial token
lifetimes and issuance are specified below; the design is specified in [email verification and recovery](email-verification-and-recovery.md)
and [ADR-022](../architecture/decisions/022-email-verification-and-password-recovery.md); runtime remains future work.

### Registration credential policy (P04.02)

The maintainer approved Argon2id hashing as a registration prerequisite before
login implementation. Passwords permit 15–128 Unicode scalar values, spaces and
Unicode without composition rules; common-password case variants are screened
offline. Exact password bytes are preserved for hashing. See
[ADR-009](../architecture/decisions/009-registration-credentials.md) for parameters,
input scope, dependencies and limitations. `POST /auth/register` persists a pending
identity and credential atomically, with the same generic accepted response for
new and duplicate identities. See [ADR-011](../architecture/decisions/011-bounded-registration-http.md).

### Initial login (P04.03)

`POST /auth/login` verifies existing exact password bytes without reapplying the
signup minimum-length or common-password policy. Pending and verified identities
may receive independent opaque access (15 minutes) and refresh (30 days)
credentials. Only digests are persisted, atomically with a session, after a locked
credential recheck. Login grants no financial authority or verification state.
Registration and login share bounded off-executor password admission. See
[ADR-015](../architecture/decisions/015-secure-login-http.md) for response and
failure behavior. Email remains future work; bearer middleware is described below.

### Sliding refresh and revocation (P04.04)

`POST /auth/refresh` consumes an opaque refresh credential once and atomically
issues a new 15-minute access / 30-day refresh pair for the same session. Every
successful refresh restarts the 30 days; there is no absolute family-age cap.
Ordinary API activity alone does not extend the deadline. This is the approved
convenience tradeoff until separately designed passkey authentication exists.

Reusing any consumed generation revokes only that family, including its current
successor, without affecting other sessions. Concurrent refresh or retry after a
lost successful response can therefore require fresh login. Consumed history is
retained; only digests are persisted. Existing access rows remain, but the
bearer middleware rejects expired tokens and revoked families. Pending users
gain no verification or financial authority. See [ADR-016](../architecture/decisions/016-sliding-refresh-rotation.md)
and [ADR-017](../architecture/decisions/017-refresh-http.md). Logout/global endpoints are implemented in P04.11 (see [ADR-021](../architecture/decisions/021-session-logout.md));
passkeys and financial authorization remain separate work.

### Authenticated identity (P04.05)

Reusable route-scoped middleware validates the existing access digest, expiry
and owning session's revocation state, then attaches a typed server-derived
user/session identity. Pending and verified users authenticate alike; identity
alone grants no financial permission. No protected production endpoint is added.
Authentication does not rotate/renew credentials or mutate persisted state.
See [ADR-018](../architecture/decisions/018-authenticated-identity.md) for header
policy, generic rejections, storage failures and revocation snapshot limits.

### Email verification and password recovery design (P04.12)

The maintainer approved verification requiring both the received token and an
active session of the same account; opening a link on another device requires
login there. Successful recovery revokes every session and pending recovery link,
requires ordinary login, and preserves the existing verification state.
The [protocol](email-verification-and-recovery.md) defines future lifetimes,
resends, storage/lock order, delivery/privacy boundaries and acceptance cases.
No challenge tables, email delivery, verification/reset routes or financial gates
are implemented by this design. Keep the implemented OpenAPI contract unchanged.

## User identity and ownership

Give each user a stable, server-generated internal identifier. Financial records
reference that identity, not the email address or a client-submitted owner. Email
is a login/contact identifier and must not become the accounting ownership key.
Schema design must keep credential and verification data conceptually separate
from financial account records, even if some identity fields share a table.

Email verification confirms mailbox access. It does not authenticate later
requests or authorize access to another user's data. Every financial read/write
still requires an authenticated user and server-side ownership enforcement.
The verification gate is server-enforced, not merely a hidden client button.

A pending registration may complete verification or request a resend through
the eventual restricted flow; it receives no financial authority from registration
alone. A verified user must also satisfy the confirmed-profile-zone requirement
when an operation needs a financial date, per [date semantics](dates-and-time-zones.md).

## Security responsibilities retained by Cuenora

- Use Argon2id for passwords, as already required by P04.03, with a unique salt
  and a maintained implementation. Store the encoded hash and its parameters;
  never a plaintext or reversibly encrypted password. Parameter selection and
  benchmarking belong to the implementation. See [OWASP password storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html).
- Design verification and recovery challenges as random, purpose-bound,
  single-use, expiring secrets. Store only a secure verification representation;
  never log raw secrets or complete links. Bind successful verification to the
  correct user and email. Exact generation, storage and consumption protocols
  require tests before implementation.
- Keep email comparison consistent across registration, login and recovery.
  Preserve the original address and define canonical comparison before adding
  uniqueness constraints. Do not silently remove provider-specific dots/tags
  or assume all mailbox local parts are case-insensitive. See [OWASP email handling](https://cheatsheetseries.owasp.org/cheatsheets/Email_Validation_and_Verification_Cheat_Sheet.html).
- Implement bounded registration/login/recovery flows, rate limits and safe
  responses that do not disclose registered users. Sessions need expiration,
  rotation/reuse handling and revocation, including logout and global revocation,
  as specified in P04.04 and P04.08–P04.12.

Owning credentials reduces dependency on an external identity provider but keeps
password, recovery, session and email-delivery responsibilities in this project.
Do not treat this choice as a substitute for implementing those protections.

## P03.04 storage decisions

| Area | Required technical decision or proof |
| --- | --- |
| Users and profiles | Server-generated UUIDs, nullable unconfirmed profile zone and verification instant, microsecond audit instants and user ownership references. |
| Credentials | Original input plus validated ASCII transport address; generated lowercase unique key; bounded Argon2id representation. Application validation and hashing are implemented in P04.02. |
| Sessions | Opaque server-tracked refresh sessions; retained digest generations, expiry, family revocation state and same-session replacement links. P04.03 adds initial access/refresh issuance; P04.04 adds sliding rotation and family revocation; P04.05 validates access identity on explicitly protected routes. |
| Email challenges | Challenge protocols are designed in P04.12; tables remain deferred to a later focused migration. |
| Database checks | Fresh isolated PostgreSQL migrations, duplicate rejection, relationship/storage guards, unchanged state after rejected writes and atomic DDL failure. |

See [ADR-005](../architecture/decisions/005-identity-storage.md) and
[identity storage](../architecture/identity-storage.md) for the implemented schema
and its application boundaries. Minimal storage does not implement Phase 04.

## Acceptance boundaries for later implementation

| ID | Scenario | Required outcome |
| --- | --- | --- |
| ID-01 | An eligible person registers without an invitation | Registration may create a pending identity under the documented email/password policy. |
| ID-02 | An unverified user attempts to create a financial account or post a movement | Server rejects access; no financial records or balance changes. |
| ID-03 | A valid verification challenge is consumed, then replayed | Only the bound email/user becomes verified; replay cannot repeat or transfer verification. |
| ID-04 | Verification is expired, invalid or for the wrong purpose/user | No verification or financial state changes. |
| ID-05 | Verified user B requests user A's financial data | Ownership checks deny access regardless of verification status. |
| ID-06 | Registration, login and recovery use equivalent email representations | All apply the same explicitly documented comparison policy; no accidental duplicate identity or account linking. |

Registration (ID-01) and the registration/login identity-consistency portion of
ID-06 have executed tests. Recovery, verification and financial authorization
boundaries remain future runtime acceptance work; P04.12 documents their design only. Registration issues no
sessions or verification challenges; the separate login endpoint issues initial
sessions. P04.05 supplies reusable identity middleware, not ownership or
verification gates. No email sender or financial authorization route exists. User,
credential and access/refresh-session tables are defined in migrations.

See the [execution plan](../plans/completed/2026-10-24-first-party-identity.md) for
the observed documentation checks and remaining work.
