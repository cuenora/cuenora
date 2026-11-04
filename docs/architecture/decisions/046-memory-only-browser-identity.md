# ADR-046: Memory-only browser identity

- **Status:** Accepted
- **Date:** 2026-11-03
- **Roadmap:** P04.03, P04.12

## Decision

Use the existing opaque bearer APIs from same-origin first-party login and
verification pages. Keep session credentials and email challenges only in page
memory, never cookies, localStorage or sessionStorage. The maintainer approved
having to sign in again after reload. There is no web dashboard: successful login
shows a signed-in confirmation and verification controls, not a financial UI.

Verification continues to require an active session of the account that received
the challenge. Opening a link is non-mutating; confirmation is an explicit action.
Trusted email links put challenges in a URL fragment, removed before enabling
controls. Verification links are optional, retaining terminal delivery by default.

## Tradeoffs

This avoids durable JavaScript-readable bearer storage and a new cookie/CSRF
contract. Credentials remain accessible to JavaScript while the page is active;
same-origin CSP and no third-party assets reduce, but do not eliminate, that risk.
Reload, navigation and page restoration require fresh authentication. Persistent
sessions and HttpOnly-cookie authentication need a separate design if requested.

No financial permission, API session semantics or verification binding is relaxed.
