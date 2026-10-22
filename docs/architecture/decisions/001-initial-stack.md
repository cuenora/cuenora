# ADR-001: Initial backend, mobile and delivery stack

- **Status:** Accepted
- **Date:** 2026-10-22
- **Scope:** Initial architecture baseline; roadmap P02.12

## Context

Cuenora is a personal, non-commercial, MIT-licensed finance project targeting
Android and iOS. It needs auditable financial behavior, server-enforced ownership,
and native mobile experiences within a manageable contributor workflow.

The [master plan](../../plans/master-plan.md) already proposes the stack. This ADR
records that direction and its costs; it does not implement missing components,
approve the plan's financial working assumptions, or select monetary precision,
currency scale, rounding, authentication, or deployment mechanisms.

## Decision

### Repository and backend boundaries

Keep one monorepo and start with a modular monolith. Use Rust, Axum and Tokio for
the HTTP API, with deterministic domain rules independent of HTTP and persistence.
The existing `cuenora-domain` and `cuenora-api` crates are sufficient now; introduce
additional crates or abstractions only for a demonstrated boundary or need.

This keeps cross-component changes reviewable together and keeps initial service
operation simple. Rust's explicit types and ownership fit the correctness goals,
but do not prove accounting correctness or authorization. Contributors must learn
Rust and asynchronous execution, and still test financial invariants explicitly.

### Persistence and API contract

Use PostgreSQL with SQLx and versioned migrations when persistence is introduced.
Explicit SQL and database transactions fit relational ownership and atomic
multi-movement operations. The cost is maintaining queries, constraints,
migrations and isolated real-database tests; adopting SQLx alone does not enforce
ownership or make a transfer atomic.

Keep a versioned OpenAPI contract synchronized with implemented HTTP behavior.
Financial amounts will use exact representations internally and decimal strings
with currency identifiers in JSON. The exact Rust money type and database
precision remain future decisions. The contract adds maintenance work but gives
clients a shared description without coupling their UI to server internals.

### Mobile and testing

Use Kotlin Multiplatform for reusable client models, validation and HTTP/use-case
logic where sharing is useful. Use Jetpack Compose for Android and SwiftUI for
iOS. Keep UI and platform integrations native; do not force everything into
shared code. Use `commonTest` for shared logic and platform tests for native
behavior. This reduces duplicated client logic while retaining two UI codebases,
interop boundaries, and Android plus macOS/Xcode toolchain maintenance.

Use Maestro for shared end-to-end flows when both apps expose equivalent behavior;
allow platform-specific subflows and platform tests for native integrations.
Shared flows reduce repeated scenario descriptions but cannot establish complete
platform parity or replace unit, contract and database tests.

Keep the MVP online-first. Offline write queues, conflict resolution and bank
connections are deferred; they add correctness and operational concerns beyond
the current foundations.

### Local infrastructure and delivery

Use Docker Compose for an isolated local PostgreSQL service and GitHub Actions
for checks on pull requests. Pin the Rust toolchain and commit Cargo.lock. Keep
runtime dependencies minimal, CI permissions read-only and secrets outside Git.
Docker adds a local daemon prerequisite; hosted CI and advisory checks depend on
external service availability. Exact environment versions and current commands
belong in [development setup](../../development/SETUP.md), not this ADR.

Require PRs and the established checks on main. RustSec auditing checks the full
locked dependency graph; Dependency Review checks PR dependency changes. Their
coverage and snapshot-warning limitations are documented in the
[dependency policy](../../development/DEPENDENCIES.md). Neither establishes the
absence of unknown vulnerabilities.

## Alternatives and tradeoffs

These are comparisons for this decision, not claims of previous prototypes,
benchmarks or an exhaustive technology evaluation.

| Alternative | Benefit | Why not selected for this baseline |
| --- | --- | --- |
| Multiple repositories | Independent repository lifecycles | Cross-client contract changes would need coordination across repositories. |
| Microservices | Independently deployed services | No current scale or team boundary justifies distributed operations and cross-service consistency. |
| A different server language/framework | Could fit different contributor experience | Retain the existing Rust/Axum foundation and domain boundary; no demonstrated need warrants a rewrite. |
| ORM-led persistence or an embedded database | Different query ergonomics or simpler local deployment | Prefer explicit SQL and PostgreSQL transactions; accept database operation and migration costs. |
| Fully separate native client logic | Avoids Kotlin/Swift shared-layer interoperability | Duplicates reusable client behavior and test scenarios. |
| A shared cross-platform UI | Can reduce duplicate UI code | Native Compose/SwiftUI UX is a project requirement; accept separate UI maintenance. |
| Platform-only E2E suites | Direct access to each platform's testing tools | Prefer shared Maestro scenarios where useful, retaining native tests for platform-specific needs. |
| Offline-first MVP | Operation without a network | Conflict resolution and queued financial writes are intentionally deferred. |

## Implementation status at acceptance

| Area | Observed state on 2026-10-22 | Evidence |
| --- | --- | --- |
| Rust API | Workspace, Axum/Tokio, GET /health/live and one API test | [Manifest](../../../backend/crates/api/Cargo.toml), [API](../../../backend/crates/api/src/main.rs) |
| Domain | Empty independent library; no financial models | [Library](../../../backend/crates/domain/src/lib.rs) |
| PostgreSQL | Local Compose service and isolated infrastructure smoke test; no backend connection or schema | [Compose](../../../infra/compose.yaml), [test](../../../tests/infrastructure/test_postgres_compose.py) |
| SQLx | Selected, not installed or implemented | [Workspace](../../../backend/Cargo.toml), [API manifest](../../../backend/crates/api/Cargo.toml) |
| OpenAPI | Only the implemented liveness endpoint | [Contract](../../../contracts/openapi.yaml) |
| Mobile and Maestro | Selected, not implemented; no mobile build or flows | [Planned scope](../../plans/master-plan.md) |
| CI and protection | Repository/Rust/audit checks plus PR dependency review; four checks required on main | [CI](../../../.github/workflows/ci.yml), [review](../../../.github/workflows/dependency-review.yml), [verified plan](../../plans/completed/2026-10-21-dependency-review.md) |

## Consequences and revisit conditions

Deliver small roadmap tasks with code, relevant tests and documentation together.
Keep application state and financial calculations separate from transport,
persistence and platform details as those layers appear. Do not build unused
infrastructure merely because this ADR names a future technology.

Revisit through a new ADR when actual workloads or ownership boundaries justify
service separation, native interop prevents useful sharing, a platform requirement
cannot be met by the selected tools, persistence requirements change, or offline
writes enter scope. Record the evidence and migration impact before changing the
baseline. Financial policies and schema design need their own explicit decisions.
