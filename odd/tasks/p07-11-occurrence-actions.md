# P07.11 — Skip or postpone occurrences
Objective: explicit planning-only occurrence actions. Branch: `feat/p07-11-occurrence-actions`; baseline: `46c09a9`.
Delivery: exception-ok (maintainer approved one coherent size:exception delivery; no publication). Runner: pinned Cargo + disposable PostgreSQL harness.

## Specs

- S1: User request: “Comencemos con la tarea P07.11 del plan maestro”. Roadmap: “Support skipping or postponing an occurrence.” Follow the approved lifecycle: skip only planned occurrences; postponement changes only due_on (and server audit timestamp), preserves planned status, original slot, terms and rule; neither action posts money. Preserve generation identity and existing confirmation/rule behavior.
- S2: User approved “Sí” to: “Propongo aceptar únicamente fechas posteriores al vencimiento actual, incluso si superan el fin de la regla, porque afectan una ocurrencia ya creada.” Accept any valid civil date strictly after current due_on, including dates beyond rule end; do not require a future date relative to today.
- Technical implementation assumptions: strict POST skip `{}` and postpone `{"due_on":"YYYY-MM-DD"}`, verified owner, permanent owner-wide action-key namespace separate from confirmation/rule keys, original action result replay even after later actions. 200 action result `{id,scheduled_on,due_on,state}`. Existing authentication, request-ID/error and bounded JSON conventions apply. No profile-zone requirement because no date is derived from today.

## Tasks

- T1 — S1–S2 — inline (one coupled behavior; no writer trigger) — implemented; own checks passed — commit pending: strict planning actions, additive retry migration, public HTTP and migration tests, synchronized contract/docs.
- T2 — S1–S2 — delegated verification (risk items 1/2/4: persistence, ownership, concurrency; long full infrastructure suite) — verified S1/S2 PASS, parent spot check passed — commit pending.

## Log

- L1: “Comencemos con la tarea P07.11 del plan maestro”
- L2: Parent asked “¿Confirmás esa política?” after proposing strictly later due dates including beyond rule end. User: “Sí”.
- L3: Read-only explorer mapped lifecycle RL07/RL11 and existing owner serialization. Parent checked lifecycle evidence. Main includes merged P07.10; pre-existing untracked `.atl/` preserved. No remote operations authorized. Native review globally off (disabled/unmanaged).
- L4: Implementation risk: items 1/2/4. Reuse recurring rule owner-first authorization, existing key syntax, UUID/date validation. Changes to shared helpers must preserve existing output and get differential evidence. All source-mutating formatting precedes final functional checks. One coherent unit likely exceeds 400 lines; no commit until delivery decision.

- L5: Initial two HTTP rules observed RED (routes absent); initial implementation passed six expanded cases, but the harness expected two while tests were expanded, so that harness run was not GREEN. Stable focused run then passed eight action groups plus migration. A further RED proved `[]` accepted as skip (200); added map-only deserialization preserving duplicate-field validation.
- L6: Final own focused target passed ten isolated HTTP groups (including observed-lock cancellation/revalidation) and the populated-history migration case, 2/2 harness groups in 88.096s. Cargo fmt check, final Clippy, locked workspace tests, repository check, OpenAPI YAML/local references and diff whitespace checks passed. Initial Clippy unused-must-use finding was fixed without suppression. Full suite and independent verification pending. Current forecast updated to approximately 1700 authored changes; one coupled unit; no commit yet.

- L7: Independent verifier: original full suite 74/75 passed in 1658.088s. Sole blocker: expected_tables omitted recurring_occurrence_action_keys; baseline same case passed in 31.961s. One correction batch updated catalog; scoped independent recheck passed 1/1 in 31.716s, no new sweep. Verified 65 production/build/migration files unchanged from frozen candidate.
- L8: Independent four probe groups passed (both skip/postpone FIFO orders, replay/revoked/foreign ownership with public-table hash invariants, cancellation, competing June/May forward dates). Twelve baseline/candidate normalized public HTTP outcomes matched exactly. Two queue/date probes retained as ordinary regression tests. Parent final target passed all 12 action cases plus migration (2 harness groups, 60.196s); final fmt/Clippy/workspace passed, 110 passed/0 failed/180 intentional DB ignores. Repository/OpenAPI refs/diff checks passed.
- L9: One slicing pass: shared engine, retry ledger and lifecycle contract form one coherent unit; splitting by file type is invalid, and behavior slices still exceed 400 with their required tests/docs. Recommend single size:exception delivery, not yet authorized. No commit, remote operation or publication. Pending only delivery decision; P07.12 untouched.

- L10 (2026-11-01): User replied “Sí” to “¿Aceptás una única entrega con `size:exception`, sin publicarla todavía?” Delivery strategy is exception-ok; local work-unit commit authorized, no push/PR/merge.
