# ADR-045: Session-bound verification mail and terminal-first confirmation

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

P04.12 approved a session-bound, single-use email verification design, but its
runtime and delivery were missing. The maintainer approved Resend with two process
environment settings and a terminal-only first implementation. No client landing
screen exists, so a button pointing at an invented URL would be misleading.

## Decision

Implement request/confirm POST routes and a bounded volatile queue. Persist only
SHA-256 digests of independent OS-random 256-bit `ev1_` challenges. Bind tokens to
user, canonical email and positive credential revision. Lock credentials/user,
challenge rows in stable order and the authorizing family, then recheck bearer
expiry/revocation and token eligibility. Consume/set verification/invalidate live
siblings atomically. Reject wrong account, purpose, expiry and reuse without
mutating state. New requests do not invalidate older unexpired tokens.

Keep the approved fixed24h lifetime, live cap5, send cooldown60s and five/hour
quota; enforce account quotas under the database lock across processes. Existing
proxy-aware IP protection gains separate request5/min and confirm20/min budgets.
The queue holds at most100 waiting identities and runs two workers; raw tokens are
created by workers and never stored in durable jobs or logs.202 is bounded
acceptance, not delivery. No provider I/O occurs inside a database transaction.

Use a narrow reqwest0.12 rustls client rather than a shell `curl` process or a
provider-wide SDK. Disable redirects/environment proxies, use a fixed HTTPS
Resend endpoint, bounded5s attempts (three max) and20s overall deadline. Retry
transient errors with identical bytes and an opaque challenge-ID idempotency key.
Provider bodies and errors never enter logs/public errors. The dependency supports
TLS timeouts and asynchronous I/O; this does not introduce a dotenv loader.

Validate paired `CUENORA_RESEND_API_KEY`/`CUENORA_EMAIL_FROM` without secrets in
errors. Both absent keeps existing startup behavior and disables mail requests.
Provider-domain verification, sending-only key scope and tracking disablement
remain explicit operator prerequisites; there is no fabricated per-message
tracking switch. Resend necessarily receives the message and token.

Use the Pen verification/base design with mail-client-safe inline HTML and a
plain-text alternative. Omit optional link/button blocks for now; put a copyable
token in the body, never subject/preheader/URL. Keep the palette, wordmark, layout,
24h warning, same-account requirement and footer. No external images/fonts/scripts.

Provide a hidden-prompt terminal helper. Its optional local mode reuses the
existing disposable PostgreSQL harness, registers/logs in through HTTP and cleans
up its own temporary resources. It refreshes the account session before explicit
confirmation, then proves token reuse is rejected. No credentials in argv/history.

## Consequences

Process loss can discard accepted jobs; retry later without revoking older links.
Existing tokens and financial authorization gates are not bypassed. Credential
revision is introduced additively; future password replacement must increment it.
Provider acceptance and fake-provider tests do not prove mailbox receipt. Live
sending requires the maintainer's own configured terminal run; never ask for keys
or tokens in chat. Client landing and password recovery remain separate work.
