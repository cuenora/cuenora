# ADR-018: Authenticate opaque access credentials at protected routes

- Status: Accepted; implemented, independently verified and native review acknowledged
- Date: 2026-10-27
- Related: [ADR-013](013-access-token-storage.md), [ADR-016](016-sliding-refresh-rotation.md)

P04.07 [ADR-019](019-typed-api-errors.md) supersedes only the legacy error-body
shape below; status codes, stable error codes and security headers are unchanged.

## Decision

P04.05 provides reusable, opt-in Axum middleware in `auth_identity`. Apply
`from_fn_with_state(pool, require_identity)` as a route layer to protected routes;
handlers extract `Extension<AuthenticatedIdentity>`. The identity has private
fields and read-only user/session ID accessors, populated from storage, never
caller-provided IDs. It conveys neither email verification nor financial authority.

No protected production endpoint exists yet. Test-only handlers demonstrate the
boundary without introducing an unrequested `/auth/me` API. Existing liveness,
registration, login and refresh routes remain public; OpenAPI paths are unchanged.
New protected endpoints must explicitly apply this layer, enforce ownership and
verification as appropriate, and document their security requirements.

## Validation and responses

Accept exactly one `Authorization` header: a case-insensitive `Bearer` scheme,
one or more ASCII spaces, then the exact emitted `ca1_` plus 64 lowercase hex
characters. Duplicate/combined headers, other schemes, extra/trailing token
content, refresh secrets and malformed credentials fail closed. Query parameters,
cookies and request bodies are not credential sources. Hash the entire access
credential using the existing SHA-256 representation; pass only its digest to SQL.

A single read-only statement joins the access row to its owning session, requiring
access expiry strictly after the database statement instant and no session
revocation. Authentication neither rotates nor renews credentials. Refresh rotation
does not invalidate old access rows before their own expiry; family revocation
invalidates every generation. No caching, schema change or dependency is needed.

| Status | Body / identity | Headers |
| --- | --- | --- |
| Handler response | Valid token supplies server-derived user/session IDs | `Cache-Control: no-store` |
| 401 | `{"error":"invalid_access_token"}` for any authentication rejection | `WWW-Authenticate: Bearer` if absent; `Bearer error="invalid_token"` otherwise; `Cache-Control: no-store` |
| 503 | `{"error":"authentication_unavailable"}` for missing/unavailable storage or query failure | `Cache-Control: no-store`; no authentication challenge |

Rejections never reach the handler or reflect credentials, owner hints or database
details. The uniform 401 policy intentionally does not distinguish malformed from
expired/revoked credentials in the body. The Bearer header transport and challenge
follow [RFC 6750](https://www.rfc-editor.org/rfc/rfc6750); its suggested 400 for
malformed requests is not used by this deliberately uniform rejection policy.
This is not an OAuth authorization server. Real deployment requires TLS; the
documented loopback HTTP server is not a production deployment.

## Snapshot and resource bounds

The query sees committed family revocation in its database snapshot. Revocation
committed after that snapshot cannot retroactively cancel an admitted handler.
Future writes must enforce their own transaction/ownership invariants; middleware
does not provide serializable authorization for a whole business operation.

Use the existing configured pool: four connections, three-second acquisition,
five-second statement and two-second lock timeouts, with statement logging disabled.
Invalid header shape is rejected before storage access. Rate limits remain P04.08;
the pool bounds database work, not total concurrent HTTP requests.

## Verification and alternatives

Public-interface tests exercise pending/verified users, independent users/sessions,
forged IDs, strict headers, expiry, refresh-purpose confusion, replay/explicit
revocation, unavailable storage and unchanged database snapshots. The isolated
PostgreSQL harness asserts all three DB groups execute; ordinary Cargo tests run
the transport cases and the integration example compiles as a doctest.

An identity endpoint would make manual demonstration easier but expand the public
contract without a current requirement. A global layer would break bootstrap
auth/liveness routes. Cached or self-contained validation would reduce lookups but
lose the existing opaque family's immediate committed-revocation check. Keep
those tradeoffs explicit rather than changing issuance or revocation policy.
