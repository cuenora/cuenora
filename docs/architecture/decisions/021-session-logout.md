# ADR-021: Logout and reauthenticated global revocation

- **Status:** Implemented in `392cb27`; own/independent verification passed; native review acknowledged, publication pending
- **Date:** 2026-10-28
- **Roadmap:** P04.11

## Decision

Keep the existing unversioned authentication routes and opaque token scheme.
`POST /auth/logout` accepts only `{ "refresh_token": "cr1_<opaque>" }` in
at most 8192 bytes of JSON. Reuse `session_store::revoke`: possession of any
retained generation, including expired/consumed ones, revokes only its family.
Return empty 204 for known, already-revoked, unknown and malformed secrets in a
well-formed payload. Repeated logout does not change the original revocation
instant. Strict payload errors are 400/413/415; unavailable storage/admission is
503 `authentication_unavailable`. No access token is needed.

`POST /auth/logout-all` requires one valid bearer access credential and strict
JSON `{ "password": "<current password>" }`. Resolve the user/session through
existing middleware, never body/header owner hints. Verify exact password bytes
with the existing bounded Argon2id verifier off the async executor, sharing
registration/login's two hashing slots. Keep admission through persistence and
retain the blocking-work permit even if the HTTP future is cancelled.

Recheck the credential hash/revision while locking the same credential row as
login. In fresh READ COMMITTED statements, acquire all that user's family locks
in UUID order, recheck that the authorizing family is active, and atomically
revoke all active families, including the caller. Preserve credential/token
history and existing revocation timestamps; no migration is needed. A login
ordered before this lock is included; one ordered after commit may create a new
session. Family locking also serializes refresh and single-session logout.
Wrong/stale passwords or a newly revoked caller fail with generic 401
`invalid_credentials`; invalid bearer uses the existing 401 challenge contract.
Storage/hash/admission failures use 503 `authentication_unavailable`. Success is
empty 204, not a replacement session. A repeat after success needs a new login.

All logout responses use `Cache-Control: no-store` and the existing server
request-ID policy; no cookies or raw credential logging. Keep the ADR-020 trust
boundary and separate fixed-window attempt keys: logout 60/IP/minute,
logout-all 20/IP/minute and 10/authenticated user/minute across IPs. The user key
is a process-keyed hash, not a raw identifier; this budget is separate from login.
Logout shares refresh's two DB-admission slots; global logout shares password
admission. Overload may return safe 503; clients must clear local tokens after
logout and treat lost commit responses as ambiguous.

## Tradeoffs and limits

Refresh possession allows logout after access expiry but means a stolen retained
refresh can terminate that family. Requiring the current password for global
logout adds friction and hash cost but prevents a stolen access token alone from
terminating every session. This is our application of [OWASP reauthentication
advice](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html#require-re-authentication-for-sensitive-features),
not a claim that OWASP mandates this exact logout contract.

Global revocation is not password recovery and does not prevent later password
logins. Already admitted handlers are not cancelled (ADR-018). Row locks can
contend/time out; every failed transaction rolls back, while a response lost
around commit may still correspond to a completed revocation. Process-local
limits remain single-instance only. Financial ownership and email verification
are not implemented by logout.
