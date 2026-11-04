# P14.02 — Liveness, readiness and container health
Branch: feat/health-readiness · Base: 28edd6d · Route: inline (one dependent feature; no parallel writer unit).
Delivery: ask-on-risk · Forecast: approximately 600 authored changed lines; no publication/deployment authorized. Runner: Cargo, isolated PostgreSQL harness and Docker runtime harness.

## Specs

- S1: User: “Pues avancemos con eso”, accepting the proposed “P14.02: liveness, readiness y health checks”. Preserve existing `/v1/health/live` status/body and its independence from PostgreSQL. Existing authentication/financial behavior remains unchanged.
- S2: Accepted proposal: “Readiness: puede atender operaciones; PostgreSQL responde y el esquema requerido está disponible.” Add a bounded, read-only public readiness check with generic output and no secrets/schema diagnostics. Assumptions: GET `/v1/health/ready`, 200 `OK` or 503 `Unavailable`, no-store, one-second overall check deadline, fail-fast single-flight admission. Probe required relations and selected compatibility columns; explicitly do not claim migration checksum/history or full operational readiness.
- S3: Accepted proposal: “configurar la comprobación en Docker/Dokploy y probar que readiness falla cuando corresponde, sin exponer información interna”. Add an internal liveness probe to the existing non-root image. Reuse listen-address validation without reading other runtime credentials. Assumptions: separate binary, no redirects/environment proxies, two-second local HTTP deadline; Docker health uses liveness, readiness is for traffic/admission and must not cause DB-driven restart loops.
- S4: Accepted proposal: “No pondría la dependencia de PostgreSQL en liveness”. Verify missing pool, unavailable DB/schema and deadline behavior against isolated state; unchanged liveness under failures; Docker status/probe and OpenAPI/docs stay synchronized. Do not modify the public staging deployment, create real user data, add migrations/dependencies or mark deployed orchestration verified without proof.

## Tasks

- T1 · S1/S2/S4 · inline · verified locally · RED/GREEN observed; six isolated DB cases passed · commit: pending — readiness endpoint and isolated HTTP/schema tests.
- T2 · S1/S3/S4 · inline · verified locally · RED/GREEN observed; Docker healthy without DB · commit: pending — local liveness probe/image/runtime tests and operational docs.
- T3 · S1–S4 · independent verification (Risk items 3/4/5: shared routing, timeout/admission and container lifecycle) · verified locally S1–S4 · checks passed; commit: pending.

## Log

- L1: User: “Pues avancemos con eso”. Prior accepted recommendation distinguished process liveness from PostgreSQL/schema readiness and proposed Docker/Dokploy configuration without DB-driven restart loops.
- L2: Baseline inspected: existing live route is unconditional `OK`; pool is optional/lazy; no readiness route, migration ledger or Docker HEALTHCHECK. Migrations 0001–0023 create 25 tables. Staging initially returned healthy liveness with an empty DB; readiness addresses that failure mode.
- L3: Implementation assumptions are scoped in S2/S3. Runtime health configuration and docs can be prepared locally; actual Dokploy settings/public alpha rollout remain an operator step. No remote credentials or server state will be accessed.

- L4: RED observed: missing readiness route failed two HTTP cases; absent probe binary failed three integration groups. Focused GREEN after implementation: library 46, probe unit 1, probe integration 3, readiness non-DB 3 passed; four DB cases require the explicit isolated harness.
- L5: Readiness uses a read-only transaction with 750ms local statement/lock limits inside the one-second overall deadline. Dedicated probe connections close on drop to avoid returning pending work after cancellation; successful probes also close. Existing configuration parser is reused through a lookup that exposes only the listen address, not dependency credentials.
- L6: Final source normalization ran before the delegated full-check batch. Full Cargo/Clippy, isolated PostgreSQL and Docker checks are running with external scratch/cache and anonymous registry config. No staging or remote provider operations are involved.

- L7: Full workspace tests passed. Initial Clippy failed only the test bootstrap `option_env!(...).expect(...)`; replaced with `env!` after the new binary existed, with no warning suppression. Final Clippy/all-targets, formatting and workspace tests passed.
- L8: Isolated PostgreSQL feature cases passed; independent S1–S4 verification added read-only-role/permission and cancellation probes, six DB cases passed. Retained those probes in regression tests and added delayed-body timeout coverage. Final explicit six-case PostgreSQL harness passed; Docker runtime HEALTHCHECK passed with no database and ignored dependency credentials. No runtime behavior changed after the passing Docker build; later changes were test retention/lint only.
- L9: Independent verification found no blockers. Parent spot-check passed. P14.02 build deliverable checked locally; actual new-image publication/deployment and Dokploy settings are pending and not authorized here. Public alpha.2 does not contain the new endpoint/probe.
- L10: One coherent readiness unit (endpoint, schema/cancellation tests, contract and docs) exceeds the review heuristic; no code/test/doc split is justified just to reduce line count. Size/delivery decision is required before work-unit commit under ask-on-risk.
