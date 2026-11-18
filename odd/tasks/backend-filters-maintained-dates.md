# Extend transaction filters and replace unmaintained date dependencies
Objective: compatible backend filters and maintained date/time implementation. Branch: feat/backend-filters-maintained-dates; baseline: c43ef9e.
Delivery: two separate review units; maintainer accepted a size exception for the migration. Runner: pinned Rust 1.99.1 and disposable PostgreSQL harness.

## Specs
- S1: "Añadir a los filtros que ya tenemos para listar las transferencias el filtrado por tipo y el filtrado por moneda que los necesitamos." Extend GET /v1/transactions with optional kind and currency, composing with existing filters and cursor admission. Supported values follow existing event kinds and SEK/EUR/USD.
- S2: "Solucionar el problema que tenemos porque la auditoria del backend detecta dos dependencias que no estan mantenidas, si es verdad que no están mantenidas debemos sustituirlas por unas que si manteniendo el contrato que tenemos ahora en los endpoints." Confirmed RUSTSEC-2026-0543 and RUSTSEC-2026-0555; replace chrono and chrono-tz, preserve date parsing, IANA zones, endpoint formats/errors and financial semantics.
- S3: "Sí" to "¿una transferencia EUR → SEK debe aparecer al filtrar por cualquiera de las dos monedas?" Match either side exactly once.

## Tasks
- T1 [S1,S3] delegated: verified; transaction filters with contract/docs/tests; parallel independent writer reason; commit 47db380.
- T2 [S2] delegated: verified; Jiff date migration with bundled IANA data, regression tests and ADR; parallel independent writer reason; commit: fix(backend): replace unmaintained Chrono dependencies. Risk: item 5 (dependencies), item 3 (shared date helpers).
- T3 [S1-S3] parent + independent verifier: checks verified; delivery decision pending; full checks, isolated database acceptance, audit, integration and delivery slicing; commit pending.

## Log
L1 (2026-11-18): Original request:
"Tenemos que arreglar dos cosas en el backend.

1. Añadir a los filtros que ya tenemos para listar las transferencias el filtrado por tipo y el filtrado por moneda que los necesitamos.
2. Solucionar el problema que tenemos porque la auditoria del backend detecta dos dependencias que no estan mantenidas, si es verdad que no están mantenidas debemos sustituirlas por unas que si manteniendo el contrato que tenemos ahora en los endpoints. El error que nos da el CI es el siguiente:

Run cargo audit --file Cargo.lock --deny warnings
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1533 security advisories (from /home/runner/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (226 crate dependencies)
error: 2 denied warnings found!
Crate:     chrono
Version:   0.4.45
Warning:   unmaintained
Title:     chrono is unmaintained
Date:      2026-09-14
ID:        RUSTSEC-2026-0543
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0543

Crate:     chrono-tz
Version:   0.10.4
Warning:   unmaintained
Title:     chrono-tz is unmaintained
Date:      2026-09-14
ID:        RUSTSEC-2026-0555
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0555

Error: Process completed with exit code 1."
L2: Both advisory pages verified via HTTPS; all versions unmaintained, no patched version. Explorer mapped shared SQL FILTER and cross-cutting date uses. Jiff bundled IANA data chosen to retain embedded timezone availability; preserve exact 0001–9999 ASCII BusinessDate policy. No publishing authorized.
L3: User authorized gpt-6-luna/high for explorers/writers and gpt-6.2-sol/medium for verifier; global Codex routing updated with effort unchanged; gpt-6-luna launch succeeded.

L4: Parallel writers /root/filters and /root/dates launched with disjoint edit surfaces. cargo-audit installation delegated to /root/audit_tool. RDD status: off (global), no native review will start. Docker daemon unavailable initially; requested local Docker app startup for isolated tests.

L5: T1 passed focused API test command and disposable PostgreSQL listing harness (7/7). RED observed rejecting kind before implementation. T2 passed domain tests, API library49tests, clippy and full test compilation; two full-suite attempts failed healthcheck with OS35/WouldBlock, varying tests. Parent final cargo fmt converged. Audit baseline reproduced both exact advisories. Parent owns infrastructure expected-test count6→7. T2 necessary surface expansions approved for source recurring_confirmation, savings_interest, recurrence CLI, API/domain tests, dates product spec, ADR064 (047 already occupied).

L6: Parent exact cargo audit --file Cargo.lock --deny warnings passed (exit0,228packages); fmt --check passed. Native assess unavailable because untracked scope undeclared; RDD is off, no lifecycle started. Independent verification S1/S3 met, S2 needs correction: restored pause year-zero guard, explicit bundled timezone database with Factory excluded, invalid-clock test changed to supported lower bound. One correction batch assigned dates writer. Tooling worker passed baseline/candidate healthcheck4each, full workspace tests serial, repository817files, DBprofile_zone/opening_balance/savings_forecast/account_planning. DBrecurring_generation failed invalid-clock fixture overflow; included in correction.

L7: One correction batch completed; independent verifier recheck passed all4targeted tests and S1–S3 met. Planning first-day pause200/no writes restored, explicit bundled-zone lookup survives restrictedTZDIR, invalid clock returns DateOutOfRange/no writes. API library49passed in parent spot check, cargo fmt check passed, exact cargo audit passed, repository817checkpassed. Full suite serial passed before bounded corrections; corrected paths then passed targetedDB/profile tests and Clippy; full suite not repeated. Native RDD remains off. All implementation verified; no commit/push/PR. Best cohesive slicing: filters~155authored lines, dates~415tracked authored lines plusADR; dependency lock generated excluded. Recommend two independent-to-main review units and size exception for cohesive migration; awaiting maintainer delivery decision before commits.
L8: Scratch cleanup was rejected by tool policy; retained /tmp/cuenora-audit-baseline.AYNFUA, /tmp/cuenora-healthcheck-baseline.5b78w3, /tmp/cuenora-healthcheck-baseline-full.kRRDup, /tmp/cuenora-verify.cU6SBk, /tmp/cuenora-jiff-tzdir.fGWH6z, /tmp/cuenora-jiff-tests.oYVbIo, /tmp/cuenora-advisory-0543 and /tmp/cuenora-advisory-0555. No scratch in repository.

L9: Maintainer approved two review units and the size exception for the migration. Both are local commits; RDD is off and publication was not authorized.
