# Execution plan: Liveness, readiness and container health

- **Status:** Active
- **Date:** 2026-11-04
- **Roadmap task ID(s):** P14.02
- **Tracking issue / PR:** Not created

## Goal and user-visible outcome

Distinguish a live process from an API with reachable required PostgreSQL schema,
and provide a non-root container liveness check plus deployment instructions.
The specification/logbook is [health-readiness](../../../odd/tasks/health-readiness.md).

## Existing state and evidence

Live is unconditional 200 OK. The optional SQLx pool connects lazily; liveness
stayed successful in staging before schema initialization. Docker has no health
probe. Migrations have no recorded runtime history/version ledger.

## In scope

Read-only readiness, bounded generic failure, unchanged liveness, a local probe
binary, container configuration, isolated tests, OpenAPI and deployment guidance.

## Out of scope

Automatic migrations, new dependencies, financial/auth changes, production or
staging operations, image publication and claims about unverified Dokploy state.

## Constraints and invariants

Readiness must not mutate data or reveal schema/credentials. Docker process health
uses liveness, not database readiness. Existing API behavior remains unchanged.

## Steps and acceptance criteria

- [x] Preserve current live contract and failure independence.
- [x] Add readiness with bounded schema/connectivity failure and no sensitive output.
- [x] Verify with isolated PostgreSQL, including missing schema and blocked queries.
- [x] Add and verify local/container liveness probe without dependency credentials.
- [x] Synchronize OpenAPI/docs and record independent verification.

## Verification

| Check | Observed result |
| --- | --- |
| Focused Rust readiness/probe/library cases | Passed after observed RED |
| `cargo fmt --all -- --check` | Passed |
| Clippy workspace/all-targets/all-features, `-D warnings` | Passed after replacing a test-only `option_env!` unwrap; no suppression |
| Cargo workspace/all-features/locked tests and doctests | Passed; ordinary Cargo explicitly ignores DB cases |
| Isolated PostgreSQL readiness harness | Six DB cases passed; including missing schema/column, locked query, SELECT permission and cancellation |
| Docker runtime harness | Passed: non-root image, HEALTHCHECK healthy without DB, probe ignores unrelated dependency configuration |
| Independent verification | S1–S4 passed, no blockers; additional probes retained as regressions |
| Repository/OpenAPI/whitespace and parent spot-check | Passed |

The unrelated full PostgreSQL infrastructure suite was not repeated. No external
staging/provider calls, new image publication or deployed-probe changes occurred.

## Risks, assumptions, and decisions

Risk items 3/4/5: shared routing, timeout/admission and container lifecycle.
Required schema probes are not migration-history, backups, mail or full permissions
certification. See the specification for chosen timing and admission assumptions.

## Completion notes

Implementation and local proof are complete. A delivery-size decision and work-unit
commit remain pending; no push or PR has been created. Keep this record active until
merge. Public alpha.2 and the actual Dokploy service have not been changed.
The master plan checks the locally verified image-build deliverable only.
