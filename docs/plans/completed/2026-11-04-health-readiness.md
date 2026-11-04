# Execution plan: Liveness, readiness and container health

- **Status:** Completed
- **Date:** 2026-11-04
- **Roadmap task ID(s):** P14.02
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/111

## Goal and user-visible outcome

Distinguish a live process from an API with reachable required PostgreSQL schema,
and provide a non-root container liveness check plus deployment instructions.
The specification/logbook is [health-readiness](../../../odd/tasks/health-readiness.md).

## Existing state and evidence

Live is unconditional 200 OK. The optional SQLx pool connects lazily; liveness
stayed successful in staging before schema initialization. At plan creation,
Docker had no health probe. Migrations still have no recorded history/version ledger.

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

The unrelated full PostgreSQL infrastructure suite was not repeated. Implementation
verification did not operate staging. Subsequent publication and operator-controlled
staging checks were separately authorized and are recorded below.

## Risks, assumptions, and decisions

Risk items 3/4/5: shared routing, timeout/admission and container lifecycle.
Required schema probes are not migration-history, backups, mail or full permissions
certification. See the specification for chosen timing and admission assumptions.

## Completion notes

Implementation and local proof completed in `d6c5dde`; the maintainer approved one
size-exception PR. PR #111 merged as `05bdd7ae38bc0eb5e75b49214b8edc0025d99f36`.
The [publication run](https://github.com/cuenora/cuenora/actions/runs/41900550725)
succeeded for both native architectures and published `carfdev/cuenora:0.1.0-alpha.5`.
Independent anonymous manifest inspection confirmed exactly AMD64 and ARM64.

Manifest digest: `sha256:b823ddb5f0921910018180b40cbff7a70895868b41815c78be11a6a7f1480b13`.

After redeploy, the operator supplied Docker status `running` / `healthy` for
alpha.5. Public HTTPS checks independently returned exact `200 OK` for live and
ready; readiness had `Cache-Control: no-store`.

In a separately approved interruption, the operator stopped only staging
PostgreSQL without removing its volume. Public readiness returned `503 Unavailable`
while liveness returned `200 OK` before and after that failed readiness request.
After the operator started the same service, both endpoints returned `200 OK`
again. No migration rerun or agent-side server mutation was performed.

This verifies the observed readiness cycle `200 → 503 → 200`, not continuous
uptime, backend restart counters, configured traffic admission, database crash
atomicity, retained financial-data correctness or restoration from backup.
Exact Dokploy/Swarm settings and the remaining Phase 14 boundaries were not audited.

Archived on 2026-11-04 after merge, publication and the scoped deployment checks.
