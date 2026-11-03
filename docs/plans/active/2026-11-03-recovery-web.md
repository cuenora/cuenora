# Execution plan: Browser password recovery

- **Status:** Verified locally — not committed or published
- **Date:** 2026-11-03
- **Roadmap task ID(s):** P04.12 client recovery follow-up
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer / agent

## Goal and user-visible outcome

User request: “Entonces pasemos a conectar la recuperacion de contraseña”.
Connect the existing request/reset pages to the existing recovery API. Deliver a
trusted first-party email link when explicitly configured. Keep verification and
browser sign-in unconnected. No live mail/provider credentials or publication.

## Existing state and evidence

Existing POST request returns generic 202; confirm returns 204 and already
revokes sessions atomically. Mail currently carries a terminal token. The
same-origin pages have disabled controls and deny connections/forms in CSP.

## In scope

- Optional validated recovery-web origin; preserve unconfigured terminal emails.
- Recovery-only progressive forms, same-origin JSON POST with omitted credentials.
- Fragment removal, memory-only token, explicit confirmation and no automatic retry.
- Generic request acknowledgement, validation, expired-link, busy, rate-limit,
  success and uncertain-outcome states. No automatic sign-in.
- Deterministic Node/Rust tests, existing isolated PostgreSQL regressions and docs.

## Out of scope

Email verification, browser login/session storage, schema/financial changes,
real mail sends, credentials, remote operations and publication.

## Constraints and invariants

Reuse existing recovery API and password policy. Never derive links from request
headers, log/store tokens, echo server error text or expose mailbox existence.
A lost confirmation response or a server failure may follow a committed reset;
show uncertainty and do not replay. GET never consumes a token. Keep disabled
HTML as the no-JavaScript default and block native form submission in CSP.

## Steps

- [x] Trusted origin and optional browser-link mail rendering; preserve terminal mail.
- [x] Connect only recovery pages with explicit client states and bounded requests.
- [x] Run focused/full checks and independent security verification; update docs.

## Acceptance criteria

Request acknowledgement is generic. Reset submits only the token and exact new
password bytes; mismatch/invalid link never submits. Only 204 means success.
Invalid/replayed links cannot reset again; success requires normal sign-in.
Server/browser tests cover rejection, rate limits, ambiguity and disabled JS.

## Verification

- Rust mail/config/identity tests; Node built-in test runner (no JS dependencies).
- Workspace Cargo tests, format, Clippy, repository validator.
- Existing disposable PostgreSQL recovery HTTP/security cases when Docker is available.
- Independent verifier for Risk items 2 and 3: recovery credentials and shared
  mail adapter/config. Native RDD remains disabled by user preference.

## Risks, assumptions, and decisions

Use optional `CUENORA_RECOVERY_WEB_ORIGIN`: HTTPS origin only, with explicit
HTTP loopback development exception. No default based on Host or API base path.
Browser sign-in is not implemented; success directs users to their usual client.
This focused change remains resumable from this request and its diff; no commit
or publication is requested. Prior dirty identity/design work is preserved.

## Completion notes

RED observed for unsafe origin acceptance, missing recovery form/CSP and missing
browser script; GREEN followed implementation. A later accessibility regression
also observed RED: Show/Hide accessible names were not restored after masking
passwords on rejection. A one-line fix and retained regression passed.

Observed verification:

| Check | Observed result |
| --- | --- |
| Node browser-boundary tests | 15 passed, including four retained adversarial probes |
| Rust config/mail/identity checks | Passed, including unchanged terminal mail and verification rendering |
| Workspace Cargo suite | 171 passed, 0 failed, 245 ignored |
| Disposable PostgreSQL recovery HTTP/security harness | Both passed (local Docker Unix socket, synthetic data) |
| Clippy, format, JS syntax, repository and whitespace checks | Passed |
| Independent security verifier and scoped accessible-name recheck | No blockers; 15 tests passed |
| Actual browser rendering | Isolated Firefox: request at 390px, reset at 1440px with synthetic token |
| Live provider/mailbox smoke test | Not run; no provider session/credential access authorized |

No store/schema or verification behavior changed. Original terminal recovery
and verification templates remain unchanged. The new web origin is opt-in;
production deployment still needs HTTPS/origin and provider link-tracking review.
No commits or publication were performed. Browser sign-in remains unconnected.
Research cross-check: [OWASP Forgot Password guidance](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html).
Temporary browser/verifier scratch directories were removed after checks; the
local API test server was stopped.
