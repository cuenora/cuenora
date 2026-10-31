# P07.03 — Recurrence storage
Objective: additive PostgreSQL planning storage. Branch: `feat/p07-03-recurrence-storage`; baseline: `19c083c` plus preserved P06.08/P06.09/P07.01/P07.02 documentation.
Delivery: exception-ok; maintainer approved one size-exception PR including prior prerequisite documentation. Runner: isolated Python PostgreSQL Compose harness. Risk: item 1 (persistent schema), item 2 (ownership constraints).

## Specs

- S1: User request: "Pasemos a la siguiente tarea"; next roadmap task: "P07.03 Migrate `recurring_rules` and `recurring_occurrences`." Scope is storage only, following docs/product/recurrence-lifecycle.md and recurrence-calendar.md; do not implement later recurrence APIs/calculation/worker.
- S2: Preserve existing financial data and enforce exact amounts, supported currencies, valid dates/states, owned account/rule/expense references, expense-only categories, stable scheduled-slot identity and confirmation shape. Planning writes create no financial effects.
- S3: Verify fresh/populated migration, rejected writes with unchanged snapshots, rollback and reverse-reference guards using isolated PostgreSQL. Runtime authentication, calendar generation, pause/resume use cases and exactly-once confirmation remain future tasks.

## Tasks

- T1 [S1–S3] inline (one coherent schema unit; no writer trigger), implemented and verified; RED/GREEN observed; complete in `f1b1e79`; approved single size-exception PR, merge not authorized. Independent verification required for Risk items 1/2. Native review disabled globally, unchanged.

## Log

- L1: User request verbatim: "Pasemos a la siguiente tarea".
- L2: Read-only explorer mapped existing constraints/harness. Existing uncommitted docs and `.atl/` must be preserved. Use composite reference keys and no posting triggers. Storage must retain excluded pause dates; generation algorithm remains P07.08.

- L3: RED observed for all three new recurrence test groups before migration: missing recurrence relations and missing migration file. No expected-failure wrapper. Added migration 0017, using native DATEMULTIRANGE for bounded closed pause exclusions plus paused_on for an open pause; no new dependency or third table. Focused GREEN pending.

- L4: Focused GREEN: three recurrence groups plus two-instance fresh-schema verification passed (4 Python tests,83.511s). Initial expected SQLSTATE assertions corrected to match identity protection (55000) and native reverse-key RESTRICT (23001); no guard weakened. Full infrastructure/Cargo suite dispatched read-only.

- L5: Full own checks passed: 67 infrastructure groups (1345.710s), Cargo fmt/clippy,93 passed/126 ignored. Independent verifier passed S1–S3 with two scratch probes and45 rejection/hash checks; no blockers. Added its distinct-occurrence/same-expense uniqueness probe; focused group passed again (20.622s). Parent fmt spot-check and repository/whitespace checks passed. Native review remains globally off.
- L6: Needs user delivery decision before commits: accumulated prior approved specs plus schema exceed400 authored lines. One cohesive slicing pass yields a prerequisite-doc slice and a migration/tests/storage-doc slice. No push/PR/merge is authorized. Prior .atl remains untouched.

- L7: User authorization verbatim: "No, crea una unica PR con excepcion de tamano usando las credenciales de gh de la sesion actual y ssh". Authorizes one size-exception PR against cuenora/cuenora main, branch push via origin SSH and current gh session; no merge. Supersedes L6 delivery block.

- L8: Prerequisite work-unit commits: 7d9b078 (P06.08/P06.09 verification), 5468310 and 83f624d (P07.01/P07.02 specs). All belong to the one maintainer-approved size-exception PR; no chain is created. Migration/test/storage documentation is the next dedicated high-risk work unit.

- L9: Implementation committed as f1b1e79 with tests and storage docs. Combined PR includes 703 additions and 11 deletions before this delivery note; size exception explicitly approved. Native review remains disabled/unmanaged. Repository validation and diff checks pass; source/migration bytes are unchanged from verified candidate. Publish only the feature branch by SSH and one PR via current gh session; never merge.
