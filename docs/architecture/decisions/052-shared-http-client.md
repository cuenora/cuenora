# ADR-052: Shared HTTP transport and typed API failures

- **Status:** Accepted
- **Date:** 2026-11-07
- **Roadmap:** P10.06

## Context

ADR-050 keeps financial models independent of transport. ADR-051 supplies exact
amount-field serializers. Mobile consumers now need HTTP transport and the
existing OpenAPI error envelope, without introducing sessions or repositories.

## Decision

Use pinned Ktor 3.6.1 with Android and Darwin engines and injectable MockEngine.
`ApiClient` accepts an origin (default `BackendConfig.BASE_URL`) and `/v1/` paths,
including encoded queries. HTTPS is required except for HTTP loopback development.
The shared `config/BackendConfig.kt` is the single build-time default for both
platforms; changing it requires rebuilding the apps, not editing each host.
Optional per-request headers carry caller-provided authorization/idempotency data;
the transport neither stores tokens nor manufactures idempotency keys.

Return `ApiResult.Success` with status, JSON tree (null for 204) and request ID.
Keep amounts as JSON strings through ADR-051 serializers; never convert via floating
point. Resource DTOs and domain mapping remain P10.08, not this transport layer.

`ApiFailure.Http` preserves status, valid diagnostic request ID, optional positive
integer Retry-After seconds and a parsed `ApiError` when its envelope is valid.
Codes remain strings so future server codes do not break decoding. A missing or
malformed envelope is still an HTTP failure, with no raw response body retained.
Null body IDs are accepted only for `request_id_unavailable`; valid header IDs
take precedence for correlation, while the parsed error retains its own body ID.
Separate invalid successful JSON, network I/O and timeout failures; propagate
coroutine cancellation and programmer errors. Never expose exception diagnostics
as typed failure messages. No payload logging is installed.

Disable redirects and install a 30-second request timeout. Do not install retry,
authentication, cookie or logging plugins. Request IDs do not prove retry safety.
`close()` releases the client and its default engine; injected engines remain
caller-owned. Engine/platform networking behavior is not a business retry policy.

## Consequences

Native artifacts and both test targets must verify the new runtime dependencies.
MockEngine checks do not prove TLS, DNS or device network behavior. Session storage,
refresh orchestration, endpoint repositories and cross-language fixtures remain
P10.07, P10.08 and P10.11. Hosts make no requests until explicitly wired later.

References: [Ktor engines](https://ktor.io/docs/client-engines.html),
[testing](https://ktor.io/docs/client-testing.html),
[timeouts](https://ktor.io/docs/client-timeout.html),
[releases](https://ktor.io/docs/releases.html), [OpenAPI](../../../contracts/openapi.yaml).
