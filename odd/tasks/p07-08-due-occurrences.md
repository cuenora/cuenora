# P07.08 — Idempotent due-occurrence generation
Objective: generate only eligible due planning slots, with no financial posting. Branch: `feat/p07-08-due-occurrences`; baseline: `8876a21`.
Delivery: exception-ok; maintainer authorized one size-exception PR without issue, current gh session and SSH push. Runner: disposable PostgreSQL and Rust workspace checks.

## Specs
- S1: "Comencemos la tarea P07.08 del plan maestro" — "Implement idempotent due-occurrence generation." Generate each absent eligible slot once as planned; preserve occurrence identity/terms/state and all financial rows.
- S2: Maintainer answered "Sí" to "¿confirmás que generemos únicamente hasta **hoy inclusive, según la zona horaria de cada regla**, recuperando vencimientos pendientes de períodos activos?" Use a trusted server clock captured after locks; inclusive local today and rule end; preserve original calendar anchor.
- S3: Honor existing inclusive generated_through, paused state and completed half-open pause exclusions. Calendar edits never backfill covered periods; ordinary active downtime does. New occurrences snapshot current rule terms, including retained archived-account references.
- S4: Reuse verified owner → rule serialization, bounded transactions and the unique slot constraint. Atomically insert occurrences and advance monotonic coverage; retries, overlap and failures cannot duplicate or partially commit. No HTTP generation route, worker, migration, dependency or posting is added.

## Tasks
- T1 | S1–S4 | inline (one coherent behavior; no writer trigger) | completed: original 6 RED → GREEN; final 10/10 passed | commit: d9e6d3200fefcd80c15a25609d6a31c34c934e5e
- T2 | S1–S4 | delegated verification (risk items 2/4: owner boundary and concurrent persisted writes) | completed: independent S1–S4 PASS | commit: d9e6d3200fefcd80c15a25609d6a31c34c934e5e

## Log
- L1: User request: "Comencemos la tarea P07.08 del plan maestro".
- L2: Horizon question above answered "Sí". Existing P07.07 merged at baseline; `.atl/` is pre-existing untracked and preserved. RDD globally off; no remote operations authorized.
- L3: Internal Rust entry point receives server-derived AuthenticatedIdentity and a trusted clock closure, evaluated after locking. Reuse existing begin/load/local_day and domain next_occurrence; no new transport surface. Risk: items 2 and 4. Functional tests use the callable application interface and disposable PostgreSQL.

- L4: Six behavioral groups observed RED with a no-op entry point (0 passed/6 failed), then GREEN (6/6) after implementation. Seven-group expansion adds all cadences, end extension and terminal preservation. Set-based insertion avoids per-date SQL; end-capped coverage preserves extension backlog.

- L5: Own verification: fmt/Clippy passed after declaration-order correction; final Cargo 106 passed/0 failed/145 ignored. Full isolated infrastructure suite 70/70 passed; final expanded generation target 10/10 passed. Repository validation and git diff --check passed. Independent fresh-copy probes passed S1–S4, clock after both locks, serialized edit/pause and cancelled wait; baseline/candidate lifecycle 9/9 each and 80 normalized HTTP responses identical. Parent repeated clock-lock probe successfully. No native review: globally off.
- L6: Retained verifier probes in the generation integration target. Production source unchanged after full suite. Cancellation evidence is limited to waiting for a lock; late-insert failure separately proves atomic rollback. One coherent work unit exceeds 400 lines; no useful independent split without separating tests/docs from behavior. Size-exception approval needed before local commit; no remote operations authorized.

- L7: User authorization: "Crea Una unica PR con excepcion de tamano usando la sesion actual de gh y ssh para hacer el push, no necesitas issue para crearla, lo autorizo". One PR against main, type:feature and size:exception; no issue or merge. Preserve `.atl/`.

- L8: Implementation and retained regressions committed in `d9e6d3200fefcd80c15a25609d6a31c34c934e5e`; all S1–S4 complete. One size-exception PR against main is the authorized delivery boundary. Native review disabled/unmanaged. Follow-up documentation commit records this evidence; P07.09 remains pending.
