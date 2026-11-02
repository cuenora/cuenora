# ADR-044: Validated startup environment configuration

- **Status:** Accepted
- **Date:** 2026-11-02

## Context

Deployment addresses and operational settings were fixed in code. Session expiry
was repeated as fixed contract values; pool timeout changes were overridden by
transaction-local SQL. The maintainer approved process environment configuration,
existing defaults, safe failure and Docker verification, without dotenv loading.

## Decision

Parse immutable `AppConfig` once before serving. Do not read environment per
request or use mutable global configuration. Keep compatibility constructors
using defaults; explicit session-policy variants receive validated lifetimes.
The same values reach persistence and response fields. Tokens already issued
retain their stored expiry. Restart replicas consistently when changing policy.

Use conservative bounded settings: access TTL 60–3600 seconds; refresh TTL
3600–7776000 seconds and longer than access. This permits shorter sessions or up
to 90-day refresh validity without accepting unbounded credentials. Defaults stay
15 minutes/30 days. Pool capacity is 1–100, timeout values 1–60000 milliseconds,
with lock timeout no greater than statement timeout. Reject malformed, empty,
overflowing and inconsistent settings; diagnostics contain only static names.

Centralize SQL timeouts at pool connection startup, including recurrence jobs;
remove local overrides. The configured pool factory is required by embedders.
Keep financial rules, Argon2, throttles and request/resource caps fixed.

Treat public API URL separately from the listening socket. A dedicated renderer
emits environment-specific OpenAPI YAML without rewriting the static development
contract. Public base URLs exclude credentials, queries and fragments. No Swagger
hosting, email link generation or deployment is implied.

## Consequences

No new dependency or dotenv loader. Docker can inject environment directly.
OpenAPI TTL fields now describe bounded values instead of constant defaults;
clients must consume response TTLs. Per-process bounds do not replace load sizing
or coordinated deployment policy. Legacy default-backed constructors continue to
provide the previous session durations and database limits.
