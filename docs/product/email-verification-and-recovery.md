# Email verification and password recovery

**Roadmap:** P04.12 · **Status:** Design only; not implemented · **Updated:** 2026-10-28

Cuenora owns email/password authentication. This design adds no endpoints,
challenge tables, sender, client screens or OpenAPI operations. Existing pending
users may log in, refresh and log out; financial access still requires separate
verified-email, profile-zone and ownership gates when resource APIs are built.

## Decisions and scope

| Policy | Decision |
| --- | --- |
| Verify email | Require both the received token and a valid session of that same account; on another device, log in there first. |
| Reset password | A valid recovery token plus a new policy-compliant password atomically changes the credential, revokes every session and invalidates remaining recovery links. |
| After reset | Normal login is required; no tokens are issued and email verification is unchanged. Pending users can recover too, then separately verify their email. |
| Identity/provider | Preserve the existing ASCII-local-part/IDNA comparison, significant dots/tags and planned Resend delivery; no automatic account linking or email changes. |

The first three rows contain explicit maintainer approvals. The protocol defaults
below are design choices, not OWASP-mandated values or measured production limits.
Lost-mailbox recovery, MFA/passkeys, changing email and support overrides are out
of scope. Mailbox compromise can defeat email recovery; this is not MFA.

## Future HTTP boundary

These are proposed versioned routes consistent with ADR-023, **not
implemented API promises**. Add OpenAPI/examples with their implementation.
JSON is strict, unknown/duplicate fields are rejected, bodies are at most 8192
bytes, errors retain the existing safe four-field/request-ID contract, and every
response uses no-store. No credentials in cookies, URL queries or error details.

| Proposed POST | Input/authority | Outcome |
| --- | --- | --- |
| `/v1/auth/email-verification/request` | Valid bearer, including pending identity; empty JSON `{}`; derive email/user server-side. | Generic 202 `{"status":"accepted"}` for eligible or already verified users; resend uses this same operation. |
| `/v1/auth/email-verification/confirm` | Valid bearer plus `{"token":"ev1_<opaque>"}`. | Empty 204 only after matching-account/email confirmation; no login or session extension. |
| `/v1/auth/password-recovery/request` | `{"email":"person@example.test"}`; no bearer required. | Same generic 202 for syntactically valid known/unknown addresses and delivery suppression; no account or session changes. |
| `/v1/auth/password-recovery/confirm` | `{"token":"pr1_<opaque>","new_password":"<new password>"}`; no existing session/password required. | Empty 204 after atomic reset; no replacement access/refresh tokens. |

Invalid JSON/email/password policy uses 400; overlarge body 413, media type 415.
Invalid/expired/used/superseded/wrong-purpose or mismatched-account challenges
share generic 400 `invalid_challenge`, with no consumption or account mutation.
Bearer failures retain the existing 401/challenge contract. Failures before
acceptance (admission/configuration) and confirmation hash/entropy/storage failures
return generic 503; IP attempt exhaustion is 429 with Retry-After. Worker failures
after 202 cannot change that response. Do not expose provider errors.
A request's 202 acknowledges bounded acceptance, **not delivery or account existence**.

## Challenge lifecycle

Generate independent 256-bit OS-random secrets, encoded as 64 lowercase hex
characters after `ev1_` or `pr1_`. Hash the complete purpose-prefixed secret with
SHA-256; token values cannot be substituted across purposes or for access/refresh.
Only token digests belong in Cuenora persistence; no raw tokens/complete links in local
logs, durable jobs, traces, analytics or diagnostic exports.

| Default | Verification | Recovery |
| --- | --- | --- |
| Absolute lifetime from committed creation | 24 hours | 30 minutes |
| Authority binding | User ID, canonical email, credential revision; bearer user must match. | User ID, canonical email and credential revision. |
| Live challenge cap per user/purpose | 5 | 5 |
| On successful consumption | Set email_verified_at once; invalidate remaining verification challenges. | Change password and revoke sessions; invalidate all recovery challenges. |

Eligibility is `created_at <= database_now < expires_at`, no used/invalidated
marker, and current bindings still match. There is no sliding extension. A new
request does not invalidate older unexpired links: otherwise repeated requests
could disable an email already in transit. At the live cap, suppress new mail
with generic 202 until consumption/expiry; this favors bounded storage over
unlimited resends. Expired/invalidated slots are reclaimable without removing
credential, session or financial history. Do not replay a token as success or
revoke sessions merely because an invalid/used challenge is submitted.

A password reset changes credential revision, also invalidating older verification
links. The user requests a fresh verification link after normal login. This does
not clear an existing email_verified_at or silently verify a pending account.

## Future persistence and transaction order

A focused migration will introduce purpose-bound challenge rows: ID, owning user,
email binding, credential revision, unique 32-byte digest, UTC creation/expiry,
nullable consumption/invalidation instants; constraints and bounded lookup/expiry
indexes. Add a monotonic credential revision to password_credentials; increment
it on every future credential replacement, with checked overflow. Exact DDL and
cleanup retention need implementation tests, not edits to old migrations here.

Reuse Email parsing, registration password validation/hashing, bounded off-executor
Argon2id admission, token entropy primitives and typed server identity. Hash a new
password before the transaction, retaining its admission permit; never hold DB
locks during hashing or email I/O. Do not authenticate recovery via the existing
logout-all HTTP handler: its bearer/current-password authority is different.

All issue/consume operations first lock password_credentials for the user, then
challenge rows in stable ID order. Session revocation additionally locks that
user's families in UUID order, matching login/global-logout ordering. Verification
also locks its authorizing family before the active-session recheck. After any
wait, use fresh READ COMMITTED statements and database time; recheck credential
revision, current email, token eligibility and the verification caller's active
session. A mismatched verification caller must not consume someone else's token.

Verification atomically consumes the challenge, sets verification only for the
bound account and invalidates sibling verification challenges. Recovery atomically
consumes the token, replaces the Argon2id hash, advances revision/timestamps,
invalidates recovery links and revokes every active family, preserving history
and existing revocation instants. Persist a token-free password-reset notification
event in the same transaction; send it later to the bound mailbox. Failure or
cancellation before commit rolls everything back; lost commit responses remain
ambiguous. A retry cannot repeat the transition. No automatic login is issued.

A login ordered before reset is revoked. An old-password login waiting behind
reset must fail the existing credential snapshot recheck; a fresh login with the
new password may succeed. Refresh before reset is covered by family revocation;
refresh after it is rejected. Already admitted handlers are not cancelled. Future
financial handlers must check verification/ownership independently of bearer
identity and must not cache unverified identity as perpetual financial authority.

## Delivery, links and privacy

Use a small provider adapter for the already planned Resend service. Proposed
MVP delivery is a bounded in-memory queue (100 jobs, two workers), not a durable
plaintext-token outbox. Reserve capacity before account lookup; accept equivalent
recovery jobs for known/unknown addresses, then resolve eligibility asynchronously.
There is no fast public unknown-account path and no provider wait in the request.
Persist a digest before sending a known eligible challenge; never hold its locks
during delivery. Unknown-address jobs perform no email send or identity creation.
Process loss can lose accepted jobs/raw tokens: the user requests another email;
202 does not promise durable mail. Durable encrypted challenge delivery would be
a separate design with key management, retention and recovery obligations.

Bound provider attempts/timeouts and finish before token expiry (initial default:
three attempts, five-second timeout, at most one minute total). Retry the same
payload with an opaque challenge-ID-based idempotency key, never a secret/email
as the key; provider acceptance is not mailbox delivery or verification. Lost
provider responses may still mean mail was sent. Delivery failures do not replace
passwords, revoke sessions, verify users or invalidate older working links.
Notification retries use the token-free committed event; notification failure
never undoes a completed reset. Do not include passwords or another recovery
credential in the notification.

Build links from a deployment-owned allowlisted HTTPS origin, not Host/forwarded
headers or client redirect URLs. Carry the token in a fragment handled only by
the first-party client; never reflect it into a redirect or GET request. Landing
GET/HEAD and email-link scans are non-mutating; only explicit POST confirmation
can consume a token. Preserve it only in memory through account selection/login,
remove it from the address bar, use no-referrer/no-store, and load no third-party
scripts/analytics on these screens. API secrets and credentials stay server-side.

Disable open/click tracking/link rewriting for authentication emails. Resend and
mail infrastructure necessarily receive message content; digest-only local
storage is **not** a claim that the mail provider stores no secrets. Least-privilege
sending keys, verified sender/domain, provider data-access/retention review and
working redaction are prerequisites before enabling this flow. No provider account
or credentials were inspected. Exact origin, sender, templates, client landing
implementation and mobile app/universal-link associations remain deployment/client
work; the design is not a claim these components exist. No SMTPUTF8 expansion.

## Abuse controls and acceptance before implementation is complete

Reuse explicit proxy trust and bounded protection state (ADR-020); separate
purpose namespaces and retain only keyed identifier hashes in limiter state.
Initial per-minute IP budgets: request routes 5 each, confirmation routes 20 each.
Use one-minute mailbox/account send cooldown plus five eligible sends/hour/purpose;
unknown recovery addresses share the same canonical-email suppression logic.
Identity/mail suppression returns generic 202; IP/protection failures are independent
of account existence. Limits and queue saturation need measured timing/load tests;
no claim of constant-time responses or production capacity is made here.

| Case | Required future observable outcome |
| --- | --- |
| V1: correct token + same pending account, including another logged-in device | 204; only bound email verified; no new/renewed session or financial record. |
| V2: missing/expired/revoked bearer or user B submits user A's token | 401/invalid challenge as applicable; token and every account unchanged. |
| V3: wrong purpose, unknown/expired/used token, equality at expiry | Generic rejection; no consumption, verification or credential/session mutation. |
| V4: resend, delayed/out-of-order delivery and two concurrent confirmations | Older unexpired links remain usable until one succeeds; one transition only. |
| R1: known vs unknown/case/IDNA variants, cooldown/cap and provider failure | Equivalent accepted envelopes/headers and timing distribution; no existence oracle, duplicate identity, password or session mutation. |
| R2: valid reset for pending/verified user | 204; new salted hash/revision, all sessions and recovery links invalidated; verification status unchanged; notification event persisted; no login. |
| R3: concurrent reset/reset, reset/login and reset/refresh in both orders | One consumption; stale credentials fail; no pre-reset family survives; fresh new-password login remains possible. |
| R4: rejected password, stale email/revision, failed DB write, cancellation | Same identity/credential/challenge/session data hashes; no notification event or partial update. |
| D1: queue full/crash, ambiguous provider/DB response, retry | Bounded work, documented generic response, no account transition due to delivery; same payload/idempotency key on provider retry. |
| D2: forged Host/redirect, GET/prefetch, tracking/logging/privacy inspection | No state-changing GET, secret leakage or untrusted destination; provider/key permissions verified before deployment. |

Execute these through public interfaces and isolated PostgreSQL/fake mail adapters,
with barriers for races and snapshot comparisons for rejected writes. No real
mail, provider account or personal database is needed for deterministic tests.

## Sources and remaining work

Checked 2026-10-28: [OWASP recovery](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html),
[OWASP email identity](https://cheatsheetseries.owasp.org/cheatsheets/Email_Validation_and_Verification_Cheat_Sheet.html),
[Resend idempotency](https://resend.com/docs/dashboard/emails/idempotency-keys)
(24-hour provider deduplication, not Cuenora token validity), and
[Resend tracking](https://resend.com/blog/open-and-click-tracking).
These inform controls; lifetimes, limits, session binding and volatile delivery
are Cuenora design choices. Implementation must recheck provider capabilities.
This completes design scope only; migrations, adapters, public contract, clients,
email delivery and financial verification gates remain future verified work.
