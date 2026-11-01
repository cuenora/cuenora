# P08.03 — Savings configuration
Objective: append and query effective-dated savings terms and rates. Branch: feat/p08-03-savings-configuration.
Delivery: single-pr (maintainer-approved size exception); forecast ~700 authored lines; runner: disposable PostgreSQL harness and Cargo.

## Specs
- S1: “Comencemos la tarea P08.03 del plan maestro docs/plans/master-plan.md” — “Allow savings-account interest terms to be configured.” Add verified-owner savings configuration HTTP operations, exact rates and explicit policies. No forecasts or financial postings.
- S2: “Sí” — confirmed querying history and adding effective-dated versions, without corrections or deletions, including future versions. Independent terms and rate histories preserve P08.02 semantics.

## Tasks
- T1 (S1–S2): inline, verified — configuration API, contract, public-interface regression tests and documentation; commit pending. Writer trigger absent; security/ownership requires independent verification.

## Log
- L1: “Comencemos la tarea P08.03 del plan maestro docs/plans/master-plan.md”
- L2: User confirmed “Sí” to append/query-only scope; corrections remain deferred.
- L3: P08.02 migration exists. Configuration must not derive today, invent defaults or mutate financial records. Global RDD is off; preserve it. No remote operation authorized. Forecast exceeds delivery heuristic; chain strategy required before commit.
- L4: User answered “Sí” to “¿Autorizás una única PR como excepción de tamaño para mantener P08.03 completa?” No remote publication authorized.
- L5: RED observed: all three public HTTP groups failed before routes existed. After implementation, all three passed in disposable PostgreSQL, including final extra precision/retry checks (2026-11-01). Formatting and repository validation passed. Security/concurrency verification and backend suite pending.
- L6: Independent verification: S1/S2 runtime pass; 3 savings and 9 existing listing PostgreSQL groups pass, full workspace 111 passed/184 ignored including one scratch-only differential test. Expired/revoked auth and conflicting concurrent rates passed; probes incorporated into regression tests. Initial verifier target-dir collision caused missing-rlib doctest failure; sequential rerun passed.
- L7: OpenAPI misplaced shared response (unanchored replacement) corrected in one batch; 2 structural tests observed RED then GREEN, all 371 parsed local refs resolve, bounded verifier recheck passed. Final parent 3-group PostgreSQL rerun, Clippy, fmt and repository validation passed. RDD off/unmanaged; no remote operations. Risk: items 2 and 4. Commit pending.
