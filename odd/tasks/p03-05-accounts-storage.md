# P03.05 — Account storage and ownership
Branch: `feat/accounts-storage`; baseline: `4545720`; delivery: `single-pr`, forecast approximately 300 authored changed lines; one coherent work unit.
Runner: `python3 tests/infrastructure/test_postgres_compose.py`; local disposable PostgreSQL 18. Parent implements inline; an independent verifier runs the long suite and ownership probes.

## Specs

- **S1 — Authorized scope.** User: "Ok, seguimos entonces", following P03.05: "Create `accounts` migration and ownership constraints." Add one forward migration; no account HTTP endpoints, SQLx dependency or financial posting implementation.
- **S2 — Agreed fields.** Master plan: "Accounts have owner, type, name, currency, and active/archived status." PRD/wireframes specify Normal/Savings and SEK/EUR/USD. Use UUID ownership and the existing microsecond audit-instant conventions. Engineering storage guards bound names and timestamps; they do not decide future UI normalization or name uniqueness.
- **S3 — Ownership integrity.** Reference existing users with restrictive deletion; expose a unique `(id, user_id)` target for future composite child references. Prove invalid owners and cross-owner child relationships fail atomically without persisted state changes. Disposable probe tables exist only in isolated tests; no speculative production child table or authenticated access claim.
- **S4 — Financial boundary.** Existing project rule: "Account balances derive from posted account movements, not client-provided balances." Store no balance or opening amount. Opening postings, account editing/archiving protocols, post-history currency/type changes, financial history and interest terms remain their later roadmap units. Storage of an archive instant does not implement account actions.
- **S5 — TDD and regression proof.** Preserve the applicable strict RED → GREEN policy: write account tests first and observe missing-schema failure before SQL. Cover valid rows, storage guards, ownership references, transactional DDL failure and unchanged data on rejection. Run the existing seven tests and baseline behavior comparison with isolated synthetic data and resource cleanup.
- **S6 — Documentation and closure.** Describe the schema and its application boundary, record observed checks and assess the final normalized work-unit candidate. Do not mark P03.05 complete before proof; do not publish automatically. Preserve the separate staged P03.04 archive in the original checkout.

## Tasks

- **T1 — S1–S6:** inline, complete with focused/full proof; tests before migration, storage docs and proportional checks; commit `1a7ef54`. Risk: items 1 (migration), 2 (ownership integrity). One behavior, no writer delegation reason.
- **T2 — S1–S6:** delegated independent verification and long full suite after focused GREEN, passed; verified implementation commit `1a7ef54`. Fresh scratch state, realistic ownership/rollback probes and baseline comparison.
- **T3 — S1–S6:** inline closure and native assessment, complete for `1a7ef54`; four-lens review approved and acknowledged. Passive closure records follow; publication remains separate.

## Log

- **L1 — User request:** "Ok, seguimos entonces" after P03.04 merge/archive preparation; next roadmap task is P03.05.
- **L2 — Mapping:** Completed bounded read-only explorer (`gpt-6-luna`, high, skills paths-injected). No product question blocks storage: P05.06 owns rename/archive behavior; do not add speculative mutation triggers or name uniqueness. Parent spot-checked P05.06 in the master plan.
- **L3 — Isolation/baseline:** New home-directory worktree from main `4545720`, with its own initialized CodeGraph index. Original checkout retains staged P03.04 archive and unrelated `.atl/` untouched. Baseline tree matches PR #28's verified head; its seven PostgreSQL tests, 21 Rust tests and four hosted checks already passed. These are inherited baseline evidence, not new test runs.
- **L4 — Route/size:** Multi-turn ownership proof, independent verification and native consent need durable tracking. Parent writes one understood schema unit inline; risk requires independent verification. Forecast is advisory; keep tests/docs with behavior. No P03.04 size exception applies to this feature.

- **L5 — RED:** `python3 tests/infrastructure/test_postgres_compose.py -k accounts` exited 1: four new tests failed in 20.920 s before account SQL existed. Three reported the missing table and one the missing migration file; disposable cleanup passed.

- **L6 — Fixture correction:** First GREEN attempt passed three of four tests (44.810 s); the ownership probe incorrectly used a TEMP child referencing a permanent table. A separate isolated repro returned SQLSTATE 42P16, confirmed by PostgreSQL 18 CREATE TABLE documentation. Corrected only test probe lifetime: ordinary table in the disposable DB, explicit drop on success and transactional rollback on failure; assert absence and rejection of reparenting a referenced account. Production SQL unchanged. Source: https://www.postgresql.org/docs/18/sql-createtable.html.

- **L7 — Focused GREEN:** All four account tests passed in 47.111 s, with cleanup. Repository validation (110 visible files), changed Markdown links and diff whitespace passed. No further source refactor was needed. Long full suite and independent proof are delegated after these own checks; original Rust/SQL migrations are unchanged.

- **L8 — Independent verification:** S1–S5 and verification scope of S6 passed without blockers/advisories. Full isolated PostgreSQL suite: 11 passed in 116.625 s, including all seven existing tests, with cleanup. Two independent SQL probes passed in 18.059 s, covering complete snapshot preservation, matching/cross-owner references, referenced-account reparenting rejection and partial-DDL rollback preserving seeded users/legacy data/index. All 19 original Python functions/methods are AST-identical; every existing backend file is byte-identical to baseline. No extra Rust build or baseline suite was needed.

- **L9 — Work unit:** Parent spot check reruns the reported repository validator; structural/link/whitespace checks cover passive docs. Delivery remains below 400 lines, so one schema/tests/docs commit is the review boundary against `4545720`. Native assessment follows after the commit; no native approval, PR or push is implied. Rollback boundary is this new unit before deployment; populated account tables require controlled forward/backup recovery.

- **L10 — Committed boundary:** `1a7ef545e88fbd4fc87ce4a777c014afa5658b84` (`feat(db): add account storage and ownership constraints`), eight paths and 271 additions. Native assessment was high/review due for executable SQL and named risk item 1. Parent spot-check repository validation passed before commit.

- **L11 — 2026-10-24 native result:** User answered "1" to this candidate's granted/declined choice. Four native captures ran concurrently in provider order; all returned zero findings on immutable patches. No correction, refuter or validator ran. Exact acknowledgement returned `action: acknowledged`, `authority: burned`, lineage `review-c5c4772f9ff1114f`, target `sha256:fefeaad69ea1195c4925cc1614a9261db55367357f7416a51819b6c5e5276083`. Reviewers inspected source; runtime proof remains the separately observed tests. Lifecycle stopped after acknowledgement.

- **L12 — Closure:** P03.05 is complete locally with verified acceptance criteria; mark only this roadmap task. Hosted CI, push and PR are pending. Keep the execution plan active until its future PR merges. The original checkout's staged P03.04 archive remains untouched; P03.06 is next and has not started.
