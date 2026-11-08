# ADR-057: Shared authentication and native session coordination

- **Status:** Accepted
- **Date:** 2026-11-08
- **Roadmap:** P11.02

## Decision

Add a shared AuthRepository adapter over the existing ApiClient for registration,
login, refresh and current-session logout. Reuse RepositoryResult, shared form
validators and native SessionStore. Registration's generic acceptance never
creates a session or promises email delivery. Login accepts existing short
passwords and preserves exact submitted credentials. Tokens remain opaque;
no OwnerId is invented and no financial authorization is implied.

AuthSession owns one backend/store pair and serializes each complete operation
with a Mutex. Its observable presentation snapshots contain no credentials.
Android owns one instance for the process, not one per Activity. Operations run
in a process-owned scope, so configuration changes cannot cancel a rotation and
create another coordinator. The foreground Activity checks renewal deadlines
against a monotonic clock; the repository and transport never retry requests.

Persist only complete pairs through the existing secure store. Publish signed-in
state only after replacement succeeds. Restore by refreshing the saved pair,
because its persisted representation has no trusted expiry metadata. Renew before
the access lifetime elapses, with a 30-second safety margin. Concurrent due checks
coalesce after the first rotation updates the deadline.

Before sending refresh, remove the saved old pair. The backend consumes rotating
generations and replay revokes their family; a lost response or process death must
not leave a consumed credential available for a later automatic retry. A failed
clear blocks access and sends no refresh. After uncertain rotation, require a new
sign-in rather than replaying. Cancellation performs bounded local cleanup in
NonCancellable context, propagating cancellation afterwards. Storage failures are
visible blocked states, never absence or successful persistence.

Logout removes local credentials before attempting remote revocation. It reports
when local removal succeeded but remote sign-out could not be confirmed; it never
promises remote revocation after a network failure. Failed local clear blocks
access and offers explicit retry. Renew/logout cannot resurrect an earlier pair.

Android uses the existing branded Compose shell behind a separate session gate.
Password input uses OutlinedSecureTextField with permanently hidden characters,
non-saveable TextFieldState, and clearing on submit, mode change, background and
disposal. The production window uses FLAG_SECURE. Internet permission enables the
existing HTTPS transport; no HTTP exception or alternate backend is introduced.
The test-only foundation Activity has no production graph/network access.

## Tradeoffs and limits

Removing the old pair before rotation favors replay safety over seamless offline
restoration: interrupted renewal requires signing in again. It does not guarantee
server revocation on a lost response or terminate an orphaned server session.
There is no plaintext fallback, cross-process coordinator, secret zeroization,
automatic financial request replay or global logout. The existing financial
placeholders remain disconnected. Broader recovery and accessibility stay in
P11.11/P11.12; first-account onboarding remains P11.03.

## Verification

MockEngine pins wire contracts and strict success decoding. Common tests cover
validation, restoration, due-call coalescing, rotation/logout ordering, cancellation,
storage failures and uncertain outcomes. Android tests cover form actions and the
gate with synthetic repositories, never the configured hosted backend. Observed
commands/results belong in the P11.02 execution record, not inferred from builds.
