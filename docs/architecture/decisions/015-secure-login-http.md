# ADR-015: Expose login through shared bounded credential admission

- Status: Accepted; implemented
- Date: 2026-10-26
- Related: [ADR-012](012-login-credential-primitives.md), [ADR-014](014-initial-session-issuance.md)

P04.07 [ADR-019](019-typed-api-errors.md) supersedes only the legacy error-body
shape below; status codes, stable error codes and security headers are unchanged.

## Context

Login must reuse the existing credential verifier and session store without
turning registration into a different public contract or allowing expensive
password work to accumulate without bounds.

## Decision

Add `POST /auth/login` with the existing strict 8 KiB JSON boundary and the
existing `{ "error": ... }` error envelope. Login validates email and input
bounds but does not reapply signup's minimum-length or common-password policy.
Unknown users still perform the bounded dummy Argon2 verification; wrong,
unknown, and credential-less identities return the same `401 invalid_credentials`.

Registration and login share one capacity-two semaphore. Login acquires capacity
before database or Argon2 work, transfers the permit into `spawn_blocking`, and
holds it until session persistence completes. Cancellation cannot release
capacity while blocking verification continues.

Successful login returns an opaque access/refresh pair, `Bearer` token type and
the approved 900/2,592,000 second lifetimes. Only the successful response sets
`Cache-Control: no-store`; no cookies or financial authorization are issued.
Verification, token generation, admission and database failures use the generic
`503 login_unavailable` response. If the credential changed after verification, issuance returns no session and the route responds with
`401 invalid_credentials`; generated tokens are never returned.

## Consequences and limits

- Pending users can log in without changing verification or profile state.
- Registration's status, body and headers remain unchanged.
- Bearer authentication, refresh rotation/revocation, email and financial
  authorization remain future work; receiving tokens is not financial access.
