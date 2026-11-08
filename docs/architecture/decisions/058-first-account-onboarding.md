# ADR-058: Authenticated first-account onboarding and write recovery

- **Status:** Accepted for P11.03 implementation
- **Date:** 2026-11-08
- **Roadmap:** P11.03, including maintainer-approved email verification, profile-zone confirmation and opening balance

## Context

Login returns opaque credentials, not an owner identity or profile. Existing shared
financial adapters require a real owner snapshot. Account listing alone cannot
supply that identity or the saved zone. Guessing an owner or silently replacing a
saved zone with the device zone would violate existing contracts. Retrying a
creation with a new key after losing its response can duplicate an account.

## Decision

Add a minimal authenticated read-only `GET /v1/me`, returning only `id`,
`email_verified` and nullable `time_zone`. Reuse bearer middleware, self-only SQL
and no-store responses; do not expose email or credentials. The existing profile
PATCH remains byte-compatible. No migration, dependency or database grant change
is required. The server still enforces every financial gate at the operation.

Shared onboarding uses the existing account repository with this server-derived
owner. Each owner-bound operation rechecks the profile under the same session
mutex as resource requests, refresh and logout. Never automatically retry a write
on a renewed session. A 401 clears local authority; an offline resource failure
does not clear an otherwise valid session. UI snapshots contain no bearer tokens.

Android reuses the existing branded native shell and base components. Verification
supports authenticated request/resend, manual token confirmation, and checking
again after the existing browser same-account confirmation. It does not register
an app link or intercept browser authentication. A 202 means request acceptance,
not delivery or verification. Verification tokens are non-saveable secure fields,
cleared on submission, background and disposal. Registration still requires a
separate ordinary sign-in.

First-use Home offers account creation without claiming accounts are required for
accountless entries. Existing users skip first-account setup. An unset zone gets
only a device suggestion; a separate explicit confirmation PATCH saves it. A saved
profile zone is retained. Opening amounts use shared exact-money validation and
serialization, and dates are checked against profile-zone today when the local
zone catalog supports it; the server remains authoritative.

Before account POST, write the immutable command, owner and random idempotency
key to an AES-GCM Android Keystore-backed journal in no-backup storage. Each record
is bound to backend origin and server-derived owner with authenticated associated
data. The journal is separate from session storage and is not an offline draft.
Never overwrite an unresolved operation. An uncertain write locks editing and
new creation; recovery retries that command/key even when the account already
appears in a listing. A different signed-in owner cannot see or replay it.
Acknowledgement clears the journal; storage failure blocks new financial writes.
An explicit validation rejection permits correction, without treating a lost
response as rejection. Cancellation before financial submission makes no financial
write; cancellation after submission preserves recovery state.

## Consequences and limits

The backend must expose the new profile read before a deployed mobile client can
complete this flow. Local tests use synthetic repositories and isolated PostgreSQL;
local implementation is not deployment or hosted verification. Full account
lists/details, accountless entry forms, password recovery and app-link integration
remain outside this task. There is no plaintext fallback, backup recovery,
hardware-backed-key guarantee or cross-process coordinator. Losing app storage
also loses its local recovery journal. The existing native review switch remains
user-owned and disabled.
