# ADR-047: Independent process health and bounded schema readiness

- **Status:** Accepted
- **Date:** 2026-11-04
- **Roadmap:** P14.02

## Decision

Preserve `/v1/health/live` as an unconditional process check with its existing
200 `OK` body. PostgreSQL or mail failure must not turn it into a restart signal.

Add `/v1/health/ready` for bounded, read-only database connectivity and required
schema availability. Return only 200 `OK` or 503 `Unavailable`, with no-store and
normal server request IDs. Limit to one in-flight probe with no admission queue;
apply one second across acquiring a connection and the read-only transaction,
with 750ms local statement/lock limits so blocked server work is bounded too. Close probe
connections on drop, even when cancelled, rather than return an outstanding
query to the application's shared pool. Healthy probes also close their physical
connection; this trades connection setup cost for explicit cancellation isolation.

A zero-row UNION query resolves all 25 required public tables and selected later
compatibility columns without reading financial records. A regression inventory
checks coverage of current CREATE TABLE migrations. Keep the probe synchronized
when required schema changes. Do not introduce a migration ledger or run migrations
at startup as part of this change.

The non-root image includes `cuenora-healthcheck`, which probes local liveness
with a two-second HTTP deadline and bounded body. It maps wildcard listen addresses
to loopback, reuses application address validation, ignores unrelated runtime
configuration, disables proxies/redirects and emits only a fixed failure message.
Docker HEALTHCHECK uses liveness; orchestrators may use readiness for traffic
admission. Never treat a DB-dependent readiness failure as a process restart rule.

## Tradeoffs and limits

Readiness is not checksum/version verification, full column/type/constraint or
write-permission validation, mail delivery, scheduled-job health, restored-data
correctness or a production-readiness certificate. Concurrent health polling may
receive 503 rather than queue. Protect polling at the deployment edge as needed.
Actual deployed probes/traffic behavior must be verified after rollout; changing
the image or documenting Dokploy instructions does not configure the live server.
