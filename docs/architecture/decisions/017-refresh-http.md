# ADR-017: Expose bounded opaque refresh rotation

- Status: Accepted; implemented, independently verified and native review acknowledged
- Date: 2026-10-27
- Related: [ADR-015](015-secure-login-http.md), [ADR-016](016-sliding-refresh-rotation.md)

## Decision

`POST /auth/refresh` accepts only a JSON `refresh_token` string within an 8 KiB
body. Unknown/duplicate fields and malformed JSON are rejected; user/session IDs
are never accepted as authority. Possession of the presented secret is verified
by the existing session-store adapter, which derives its owning family from the
digest. Pending and verified users can refresh without gaining financial access
or changing identity/profile/verification state.

| Status | Body | Meaning |
| --- | --- | --- |
| 200 | Same five token-pair fields as login | New independent secrets, 900-second access and sliding 2592000-second refresh validity. |
| 400 | `{"error":"invalid_request"}` | Malformed JSON, missing/wrong-type field or unknown/duplicate fields. |
| 401 | `{"error":"invalid_refresh_token"}` | Malformed, unknown, expired, revoked or consumed secret. Consumed reuse revokes only its owning family before rejection. |
| 413 / 415 | `{"error":"invalid_request"}` | Body exceeds 8192 bytes / unsupported or missing JSON content type. |
| 503 | `{"error":"refresh_unavailable"}` | Missing database configuration, full refresh admission or storage/entropy failure. No internal details. |

All responses from this operation, including errors, carry `Cache-Control:
no-store`; no cookies or caller-provided identity fields are introduced. Empty or
wrong-purpose string credentials get the generic 401 when storage is available;
the handler does not expose storage lookup details. See the
[OpenAPI contract](../../../contracts/openapi.yaml).

## Admission and failure semantics

Refresh has two non-queued in-flight slots, separate from the existing two shared
registration/login password-hashing slots. It performs no password hashing. Its
permit stays owned through persistence, including replay-triggered revocation.
Existing database pool/acquisition/statement/lock limits remain unchanged.

The HTTP boundary calls the already-verified rotation adapter unchanged. It does
not reimplement ownership, token generation, expiry, locking or revocation.
Ordinary rejected input and failed writes leave stored state unchanged; reuse
rejection intentionally commits family revocation. Cancellation or a lost commit
response is not proof of rollback. Clients must serialize refresh and update the
pair together; retrying a consumed predecessor can require fresh login. There is
no grace period or promise of network-level exactly-once delivery.

Cancelling the HTTP future releases its admission permit, but SQLx may finish an
in-flight statement before processing rollback. For a request blocked on its
family lock, the existing lock timeout bounds that statement; cleanup can finish
after the HTTP task is gone. Do not infer immediate database quiescence or, near
commit, a guaranteed rollback from task cancellation alone.

## Limits

Revocation is exposed automatically through replay detection and the internal
possession-scoped adapter. Logout/global-revocation endpoints remain P04.11.
Bearer identity middleware and financial authorization remain P04.05/P04.06;
returning `token_type: Bearer` alone implements neither. Broader abuse/rate limits,
email and passkeys remain separate work. Sliding lifetime is the approved
convenience tradeoff, not a claim that financial tracking data is insensitive.
