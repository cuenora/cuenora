# P05.10 — Accountless expense
Record expenses without account movements. Branch: `feat/p05-10-accountless-expense`; baseline: `cb682ed`.
Delivery: exception-ok (single PR, maintainer approved); forecast 650 authored lines; runner: pinned Cargo and isolated PostgreSQL harness.

## Specs

- S1: “Record an expense without an account.” Exact positive monetary amounts, explicit business dates, optional compatible expense category and bounded description. Reuse verified ownership and confirmed-zone gates. No movement or balance effect; linked expense and reporting endpoints are out of scope.
- S2: User approved “claves permanentes separadas de ingresos” with “Si”. Retain normalized owner-scoped retries permanently; no duplicated events/audit, immutable key records, conflicts without writes, atomic rollback. Existing income requests and retries remain unchanged.

## Tasks

- T1 — S1/S2 — inline (one behavior; no writer trigger) — implementation/checks complete; commit pending. Risk: items 1–4 (new schema, ownership, shared HTTP contract, owner serialization).
- T2 — S1/S2 — independent verification (named high-risk items 1–4) — passed; commit pending.

## Log

- L1: User: “Actualiza mein local con el repositorio remoto y luego comencemos con la tarea P05.10”. User authorized current GitHub CLI HTTPS fetch; main fast-forwarded to cb682ed. Preserve untracked `.atl/`.
- L2: User approved separate permanent expense retry keys: “Si”. No publication authorized.
- L3: Reuse income transport validation, but isolate expense application/storage and preserve income fingerprints. Forecast exceeds 400; ask delivery strategy before committing. Applicable checks: RED expense creation; expense/income isolated PostgreSQL; fmt, Clippy, workspace tests, repository check; independent baseline differential verification.

- L4: RED observed: valid expense POST returned 400 instead of 201; absent expense key table. Implemented separate application module/migration; shared strict HTTP validation only. Income fingerprints/code unchanged.
- L5: Final fmt/Clippy passed; Cargo 90 passed/90 ignored; expense five PostgreSQL cases and migration upgrade passed; existing income eleven passed. Repository/whitespace checks passed. Initial synthetic-bearer unit expectation corrected from 503 to established 401 before final successful checks.
- L6: Independent S1/S2 passed: baseline-created durable income key replay, five baseline/candidate error comparisons, competing expense payloads, UUID normalization and owner-lock timeout. Committed-test probes incorporated in candidate.
- L7: Full PostgreSQL suite: 57/58 passed in 1155.020s. Missing expense_creation_keys in expected_tables caused sole failure; corrected once. Exact schema test then passed locally and independently (23.603s); no full-suite rerun.
- L8: Needs maintainer delivery-size decision before commit: approximately 900 authored lines including tests, migration, contract and documentation in one financial behavior. Native review remains enabled, not started. No push/PR authorized.
- L9: Native mode globally on. Preliminary assessment returned high/unassessable because new files are untracked; no consent or authority created. Resolve candidate inventory (exclude pre-existing .atl/) after delivery-size choice. Scratch cleanup was denied by runtime; verifier scratch remains /tmp/cuenora-verify-p0510-BAhvRq and test logs remain outside the repository.
- L10: Maintainer answered “Sí” to the single-PR size exception (~900 authored lines). Commit locally as one coherent work unit; publication remains unauthorized.
