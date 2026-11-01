# P07.07 — Recurrence rule lifecycle
Objective: authenticated creation, editing, pausing and resumption of expense rules. Branch: `feat/p07-07-recurrence-rules`.
Delivery: exception-ok, user approved one cohesive delivery with size exception (no remote publication), approximately 1,900 authored changed lines (expanded contract/guard coverage); no remote operations authorized. Runner: Rust workspace plus disposable PostgreSQL HTTP integration harness.

## Specs

- S1: User request: “Comencemos con la tarea P07.07 del plan maestro”. Roadmap: “Implement creating, editing, and pausing recurrence rules.” Implement the approved lifecycle, including resumption, without generation, worker, posting, occurrence actions or deletion. Reuse exact money/calendar/zone validation and existing storage. Enforce verified authenticated ownership. Verify through public HTTP and isolated PostgreSQL.
- S2: Proposed calendar boundary: “Recomiendo aplicarlos solo a fechas posteriores al día de edición y al período ya procesado, sin modificar ocurrencias existentes ni generar vencimientos retroactivos.” Question: “¿Confirmás ese criterio para cambios de inicio, frecuencia y zona horaria?” User: “Sí”. Preserve existing occurrences and advance the inclusive generation boundary only for actual calendar/zone changes. Verify past/covered dates cannot be regenerated and original anchors remain independent of progress.
- S3: Archived selection proposal: “Recomiendo **rechazarlas al crear o reasignar una regla**, conservando las referencias que ya existían antes del archivo.” Question: “¿Confirmás también ese criterio?” User: “Sí”. Verify create/reassignment rejection is atomic, existing archived references survive unrelated edits, pause and resumption, and foreign/mismatched accounts are rejected.
- S4: Preserve approved P07.02 lifecycle: create active; edits preserve state and occurrence snapshots; pause retains commitments; resumption preserves the original anchor and completed half-open civil-date exclusions without pause-period backfill. No financial effects. Verify repeat transitions, exclusions and financial/occurrence snapshots. API shape and retry details are implementation assumptions to document before their tests; do not silently change approved financial policy.

## Tasks

- T1 — S1–S4 — inline (one lifecycle unit; no independent parallel writer surfaces) — complete: focused plan and public HTTP RED observed (7 DB groups failed before runtime; unauthenticated route test also failed); final contract added. Verified; Commit: `17ae577da6c7bc3873cfa9c2edd65ab69132544d`.
- T2 — S1–S4 — inline — complete, independently verified; Commit: `17ae577da6c7bc3873cfa9c2edd65ab69132544d`: strict transport, serialized persistence, additive permanent creation-key migration, contract and ADR-037; Risk: items 1, 2, 4 (persisted state, ownership and serialized transitions).
- T3 — S1–S4 — inline checks plus delegated verification (named risk items 1, 2, 4) — complete: all own and independent checks passed; Commit: `17ae577da6c7bc3873cfa9c2edd65ab69132544d`.

## Log

- L1 (2026-11-01): “Comencemos con la tarea P07.07 del plan maestro”.
- L2 (2026-11-01): User approved future-only calendar edit boundary: “Sí”.
- L3 (2026-11-01): User approved archived-account selection criterion: “Sí”.
- L4: Read-only explorer mapped the lifecycle/storage and existing API helpers. Parent checked `recurrence-lifecycle.md`, `recurrence-storage.md`, migration 0017 and account-management policy. No recurrence API exists. Existing owner-first locking, `parse_zone`, `Money`, `BusinessDate`, UUID normalization and request error/auth middleware are reusable. Native review mode is globally off and remains unchanged. Existing untracked `.atl/` is preserved.
- L5: Forecast exceeds the review budget. Delivery strategy must be resolved before the first work-unit commit. One cohesive lifecycle/API unit is preferable to splitting transport from its working persistence, tests and contract. No push/PR/merge authorized.

- L6 (2026-11-01): User approved one size-exception delivery without publishing a PR: “Sí”.

- L7: RED observed before runtime: 7/7 isolated DB groups failed, plus the non-DB bearer test. Runtime now covers creation/replay, state-preserving PATCH, future-only calendar cursor updates, and atomic half-open pause unions. First focused GREEN run passed 7 lifecycle groups. Migration proof initially encountered unique-before-FK precedence in its deliberately invalid fixture; corrected the fixture to use an unkeyed second rule, without changing implementation. Added transport/currency/cadence, backwards-resume and injected ledger-failure rollback coverage; final focused rerun pending.

- L8: Final focused run passed 9/9 isolated PostgreSQL lifecycle groups plus additive migration/history/retry constraints. Full own verification passed pinned Rustfmt, Clippy with -D warnings and workspace tests; the disposable PostgreSQL harness passed all 69 groups in 1421.408s. Repository validation passed (366 visible files) and diff check passed. Ordinary Cargo intentionally ignores PostgreSQL cases. Native review remains globally off; independent verification of risk items 1/2/4 is pending.

- L9: Independent verifier passed S1–S4: 4 scratch-only public HTTP probe groups (including 12 concurrent edit/pause races and injected AFTER UPDATE rollback); 16/16 baseline/candidate responses had identical status/normalized bodies on existing routes and absent-route controls. No change-caused blockers. A pre-existing stale lifecycle status sentence was corrected; no runtime correction was needed.
- L10: Parent spot-check reran `PostgresComposeTest.test_independent_recurrence` in the isolated candidate copy: passed, 82.844s. Compose resources were cleaned. Final source bytes are unchanged since functional verification; only passive completion documentation changed.
- L11: Delivery remains one approved size-exception unit, no push/PR/merge. Approximately 1,900 authored changed lines include full contract and executable guard coverage. Rollback boundary is the new recurrence API modules, route merge, additive retry-key migration and their contract/tests/docs; populated retry/history storage requires a forward migration rather than destructive rollback. Next roadmap task is P07.08, not included. Risk: items 1, 2, 4 (persisted lifecycle and retry state, verified ownership, serialized mutation races). Native review disabled/unmanaged; no review receipt claimed.

- L12: Implementation committed as `17ae577da6c7bc3873cfa9c2edd65ab69132544d` (1912 additions / 8 deletions, 1920 authored changes). T1–T3 and S1–S4 are complete. Disposable verification scratch artifacts were removed successfully; only the pre-existing untracked `.atl/` remains. Closure documentation is a separate passive commit so the task can record the implementation identity without a self-referential hash.
