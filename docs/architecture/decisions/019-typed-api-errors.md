# ADR-019: Typed safe errors and server-generated request IDs

- Status: Accepted; own/independent verification passed and native review acknowledged for `2340d2e`
- Date: 2026-10-27
- Roadmap: P04.07
- Publication: [PR #59](https://github.com/cuenora/cuenora/pull/59); merge pending
- Supersedes: Error-body shapes in ADR-011, ADR-015, ADR-017 and ADR-018 only

## Decision

The maintainer explicitly approved replacing `{error: ...}` with exactly `code`,
`message`, `details`, `request_id`. A shared closed Rust `ApiError` enum determines
status, stable code and safe static English message. Preserve existing statuses,
codes, success bodies, Bearer challenges and route-specific no-store behavior.
`details` is always null until a safe structured detail contract is introduced.
Never serialize supplied credentials, validation inputs, database errors or traces.

Use existing OS randomness through the existing Argon2 rand_core re-export; no
new dependency is required. Generate 128 bits per request as 32 lowercase hex
characters, expose a private-field `RequestId` extension, and set `X-Request-ID`
on the response. Error JSON uses the same ID. Replace untrusted incoming ID
headers including duplicates; do not validate or preserve them. IDs are diagnostic
correlation only, never authentication, ownership, idempotency or a retry guarantee.

Wrap the final composed router with `with_request_context`; nested instances reuse
one server extension. `app(pool)` wraps its current routes and fallbacks. Route-scoped
identity middleware also establishes context for standalone use, preserving its
existing integration while using the same error serializer. Typed private handlers
explicitly consume the extension; arbitrary future handlers/extractors must map their
own failures to `ApiError` rather than assume response bodies are globally rewritten.

Normalize unmatched paths to 404 `not_found` and unsupported methods to 405
`method_not_allowed`. Keep Axum's `Allow` and HEAD body suppression. No API path,
financial endpoint, token lookup, credential rule, database write or authorization
policy is introduced or changed. P04.06 remains deferred until real resource endpoints.

## Failure boundary

If OS randomness fails, reject before application handlers with 503
`request_id_unavailable`, safe static message, null details and null request_id.
Do not fabricate an ID or reflect a caller's value; omit `X-Request-ID` and return
`Cache-Control: no-store`. This is the only application error allowed a null ID.
Transport parsing failures before Axum, disconnects and panics are outside this
contract; no catch-all exception or proxy logging system is implied.

## Alternatives and consequences

Keeping the old field would reduce migration work but conflict with the approved
four-field contract. Client-provided correlation IDs help some proxy deployments
but introduce spoofing and disclosure risks; any future trusted-proxy policy needs
its own explicit trust boundary. A new UUID/tower dependency is unnecessary for
128-bit opaque IDs. OS entropy adds a rare liveness failure, handled explicitly
rather than panicking or returning misleading correlation. We do not claim strict
mathematical uniqueness or use the IDs as secrets.

Existing error consumers must migrate to `code`; test equality normalizes only a
request ID after checking its syntax and exact header/body match. Negative HTTP
and PostgreSQL tests must still prove unchanged state. Contract, independent
verification and native review outcomes are recorded in the execution plan.
