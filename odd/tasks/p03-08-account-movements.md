# P03.08 — Account movement storage
Objective: append-only, owner-scoped account postings. Branch: `feat/p03-08-account-movements`.
Delivery: ask-on-risk; forecast ~360 authored changed lines, one work unit. Runner: isolated PostgreSQL Compose tests.

## Specs

- **S1:** “Comencemos con la tarea P03.08 del plan master”. Roadmap: “P03.08 Create `account_movements` migration, foreign keys, and indexes.” Store exact signed minor units, owned account/event references, economic date and audit instant; index owner/account/date and event lookup. Verify fresh application, references, bounds, indexes and transactional recovery.
- **S2:** User answered “`Sí`” to “mantener los movimientos originales y corregir mediante movimientos compensatorios”. Reject UPDATE/DELETE/TRUNCATE, allow linked positive/negative compensation inserts, preserve original rows. Editing/voiding commands remain P05.13.

## Tasks

- **T1 — S1–S2:** inline implementation, complete; no writer trigger (one coherent behavior). Risk: items 1 and 2 (new financial migration and ownership constraints). Own checks, independent verification and native review passed/acknowledged. Implementation commit: `fdaa7d9`. RED/GREEN: observed (L5).

## Log

- **L1:** User: “Comencemos con la tarea P03.08 del plan master”.
- **L2:** User: “`Sí`” (approval of preserving original postings and compensating entries).
- **L3:** Existing 0005 transaction migration supplies `(id, user_id)` ownership target; tests reuse disposable Compose projects, SQLSTATE checks and snapshots. Existing `.atl/` is untracked and must remain untouched. Base: `eda6604caf67806be0c9455d7a9cf9932c303b0c`.
- **L4:** Every movement is an effective posting; no independent planned/voided state. Its stored date/currency/amount is a posting snapshot. No uniqueness on transaction/account: compensations must remain possible. Cross-table currency/sign/cardinality guards remain P03.10; posting and correction workflows remain future tasks. Zero is representable for zero opening balances. Test-only direct SQL is not a posting service.
- **L5:** Initial focused RED: `python3 tests/infrastructure/test_postgres_compose.py -k movements` — four failures, all missing-table assertions (33.543s). Initial GREEN after 0006: four tests passed (113.345s). Expanded to five groups for same/cross-currency transfer storage and no automatic postings; full suite pending.
- **L6:** Full candidate PostgreSQL suite: 25 passed (406.225s). Repository check: 125 files passed. Rust 1.99.1 fmt/clippy passed; Cargo tests: 21 passed. Archived-base exact-event-storage regression: one passed (11.194s), same assertions pass in candidate suite. No existing test/helper changed. Independent verification pending.
- **L7:** Independent verifier: S1/S2 PASS, no blockers. Two fresh-scratch probes passed (32.682s), including original-row hashes, compensations, rejected mutations with preceding-write rollback, cross-owner/missing links, baseline stdout/stderr/exit equivalence and AST-identical old tests/helpers. Parent reran the exact probe command: two passed (32.520s).
- **L8:** Structural closure: 51 relative documentation links resolve; whitespace clean. Only P03.08 is checked after functional and independent proof. Plan remains active until PR merge. Native review is enabled globally; assess the normalized work-unit commit next. No push/PR/merge authorized. Rollback boundary: new 0006 schema/tests/docs before deployment; populated financial data requires reviewed forward/backup recovery.
- **L9:** Work unit `fdaa7d9`: 335 additions / 4 deletions (339 authored lines), single coherent schema/tests/docs slice. Assessment remains high/due: initial read refused undeclared pre-existing `.atl/`; obtain canonical inventory and explicitly exclude it. No native transaction has started.
- **L10:** User selected “1” / `granted`. Four native reviewer captures ran concurrently against the immutable 12-path candidate through `ab073c3`; every lens returned zero findings. No correction/refuter/validator required. Exact acknowledgement confirmed `authority: burned`, lineage `review-e67c5d17571aa4a9`, target `sha256:c291d37c1c741eea8a396c93d3ca597f676ab1a400156ff8b2c12a191d873562`, consumed revision `sha256:20cf7865238c6e066f34454de68fd1862021a5429b0e8e655556c0d48e57912b`.
- **L11:** Local implementation complete. Known verification scratch removed with non-force recursive cleanup; disposable Docker resources had already been cleaned. Only passive closure records follow the reviewed boundary; runtime source is unchanged. Publication and PR remain unauthorized; P03.09 not started.
- **L12:** User: “Crea la PR”, then “Sí” to using the current GitHub CLI session to push this branch and create the PR in `cuenora/cuenora` against `main`. Published through the authorized HTTPS credential helper and created [PR #35](https://github.com/cuenora/cuenora/pull/35), initial head `01008f8`. No merge authorized. This follow-up only records publication; migration and tests are unchanged.
