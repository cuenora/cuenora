# Execution plan: Session-bound verification mail

- **Status:** Verified locally — live mailbox test and publication pending
- **Date:** 2026-11-03
- **Roadmap:** P04.12 verification runtime follow-up; password recovery remains pending.

## Goal and scope

Implement Resend verification request/confirmation with same-account bearer
binding, digest-only challenges and a human-operated terminal smoke test. Scope,
exact user approval, task status and verification log are in
[`odd/tasks/email-verification-resend.md`](../../../odd/tasks/email-verification-resend.md).
No frontend landing page, password reset, real provider operation, remote deployment
or publication is authorized by this implementation step.

## Design evidence

Read via Pen MCP from `docs/cuenora-design.pen`: desktop verification `T23dO`,
base component `RN1jn`, and handoff text `VLREm`. The template preserves the
600px white single-column message, blue/navy palette, serif wordmark/headline,
monospace text, safe preheader, 24-hour notice, same-account sign-in explanation
and footer. The approved terminal-only scope replaces optional button/fallback
link blocks with a copyable token and terminal instruction, not a broken link.
No live secret is included in a subject/preheader, external image or URL.

## Policy and implementation

Follow [P04.12 design](../../product/email-verification-and-recovery.md) for the
verification subset: 24h, five live challenges, one-minute cooldown/five per hour,
single use, sibling invalidation and active bearer recheck after transactional
locks. Migration0022 adds credential revision and challenge storage without
changing prior credential/session/financial values. Raw tokens exist only in
process memory, the outgoing message and the mail provider's infrastructure.

Volatile queue: 100 waiting identities, two workers; process loss can discard
accepted requests. Generic202 acknowledges bounded acceptance, not delivery.
Delivery uses three attempts at most, five-second request timeout and20-second
overall limit; retry same payload and opaque idempotency key only on transient
failure. No database locks are held during provider calls. Provider errors are
not exposed. IP budgets request5/min and confirm20/min use existing proxy policy.

## Verification

Passed:8 fake-provider unit tests,7 disposable PostgreSQL store tests and5 HTTP
tests, populated migration preservation (146.563s), Cargo workspace suite,
fmt/Clippy, repository validation,7 OpenAPI structural tests and3 helper tests.
A real local helper bootstrap/register/login/cleanup test passed (107.176s),
without requesting mail or contacting Resend. Full candidate Docker release/runtime
regression passed. Independent S1-S5 security/concurrency verification passed,
including21 PostgreSQL cases across store/HTTP/login/identity (74.331s) and
independent rejected/sibling-token probes. See task log for intermediate failures
and fixture corrections; no failing run is represented as a passing run.
No live provider acceptance/inbox delivery was attempted. Native review remained
globally disabled; no native approval, commit, push or PR is claimed.

## Risks and delivery boundaries

Risk: items1,2,4,5 (persisted schema, authentication, concurrency, dependency).
Real mailbox delivery is deliberately deferred to the maintainer's terminal
session. The maintainer must disable provider open/click tracking and use a
verified sender with a restricted sending key. Provider acceptance is not proof
of inbox delivery; do not claim a live email arrived based on local tests.

## Terminal diagnostic correction

The first human test reported only a generic failure after cleanup. This did not
identify whether startup, registration or another step failed: the helper's main
handler discarded its own safe error reason. A regression reproduced the lost
message (RED); the helper now reports that fixed reason. Failed child startup
exposes only allowlisted configuration variable names, never raw stderr or values.
Five helper tests, repository validation and whitespace checks pass. The original
human failure's underlying cause still needs the next terminal run; no live send
was attempted by the agent while diagnosing it.

## Human smoke-test and publication authorization

The maintainer reported the live terminal result: confirmation204 followed by
400 `invalid_challenge` on replay, then disposable-resource cleanup. Source
inspection confirms the helper emits success only after both checks. This is
human-reported mailbox/token evidence, not agent access to the provider/mailbox.
The temporary verified account was discarded. No keys, tokens or addresses were
shared. The maintainer subsequently authorized one PR, a size exception, no issue,
SSH push and the current GitHub CLI session. Client landing and recovery stay out
of scope; no merge is authorized.
