# ADR-022: Session-bound email verification and password recovery

- **Status:** Accepted design; runtime not implemented
- **Date:** 2026-10-28
- **Roadmap:** P04.12

## Decision

Adopt the [email verification and recovery protocol](../../product/email-verification-and-recovery.md).
The maintainer explicitly approved verification using both the received token and
a session of the same account, including login on a second device. Successful
recovery revokes all sessions/outstanding recovery links, requires normal login
and does not automatically verify email.

Use independent purpose-bound, expiring, single-use digest-backed challenges.
Bind them to current email/credential revision and serialize consumption with
existing login/revocation locks. Keep the planned Resend provider behind a bounded
adapter; no secret-bearing durable outbox. Generic asynchronous acceptance avoids
coupling public recovery responses to account existence or mail-provider latency.

## Tradeoffs and implementation boundary

Session binding prevents accidental cross-account verification but adds cross-device
login friction. Revoking sessions after reset limits stale access but signs out
all devices. Retaining older unexpired resend links avoids request-based denial
of a link already delivered; lifetime/live-cap bounds limit exposure/storage.
Volatile mail jobs avoid durable raw-token storage but can be lost on restart;
a retry is a fresh request, not a durable-delivery guarantee.

The detailed document labels implementation defaults and future acceptance tests.
No existing login/refresh/logout behavior, API contract, schema or dependency is
changed here. Client landing/link association, sender configuration and provider
privacy review remain implementation/deployment prerequisites. Existing ownership
and verification gates remain required before financial use; design completion
is not Phase 04 completion or production readiness.
