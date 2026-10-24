# P03.06 — Global built-in category storage
Branch: `feat/categories-storage`; base: `c6d5de2`; delivery: `ask-on-risk`, forecast ~340 authored changed lines, one work unit, no publication authorized.
Runner: `python3 tests/infrastructure/test_postgres_compose.py`; implement inline, independent verification for Risk items 1 (migration) and 2 (write protection).

## Specs
- S1: User: “Continuemos con la tarea P03.06 del plan master”; roadmap: “Create `categories` migration with built-in categories.” Scope is storage, not P03.07 transactions or custom categories.
- S2: User approved with “Perfecto”: “Recomiendo un **catálogo global**, compartido y no editable por usuarios: evita duplicaciones; como contrapartida, personalizarlo requerirá categorías propias más adelante.” Verify shared reads and rejected mutations with unchanged stored data.
- S3: User approved with “Sí, está perfecto”: “**Ingresos:** Salary, Interest, Other income.” and “**Gastos:** Housing, Groceries, Transport, Utilities, Health, Leisure, Other expense.” Verify exact ten rows and stable identities on fresh databases.

## Tasks
- T1 | S1–S3 | inline (one cohesive understood storage unit, no writer trigger) | verified | 4 focused tests RED (22.394s), GREEN (49.198s); final suite 15 pass (167.311s); commit pending.
- T2 | S1–S3 | delegated verification (Risk items 1 and 2) | functional proof complete; native assessment pending | S1–S3 PASS, no blockers; parent spot check passed; commit pending.

## Log
- L1: User: “Continuemos con la tarea P03.06 del plan master”. Later confirmations: “Perfecto”; “Sí, está perfecto”.
- L2: Read-only mapping confirmed no existing taxonomy. Parent verified master model and future customization scope; current main `c6d5de2` is tracked-clean except unrelated untracked `.atl/`, preserved.
- L3: Use deterministic UUIDs, unique machine codes and `(id, kind)` reference target; statement trigger rejects ordinary INSERT/UPDATE/DELETE/TRUNCATE after seeding. Database owners can bypass schema guards; authentication and least-privilege application roles remain later work. SQL files use existing single-transaction runner; prior migrations must remain unchanged.
- L4: Native review mode on globally. Review consent may interrupt delivery; this document retains implementation and proof state. No push, PR or merge authorized.
- L5: Four focused category tests failed for absent migration before implementation, then passed after it. All 26 baseline function ASTs unchanged; seven functions added. Repository validation passed (116 files), staged whitespace clean. Candidate initially 264 authored insertions across eight paths. Prior migrations, Rust, dependencies and CI unchanged.
- L6: Independent verifier: baseline 11 tests PASS (118.963s), candidate 15 PASS (168.926s), additional non-owner role probe PASS (11.661s), S1–S3 PASS with no blockers. Parent reran exact probe PASS (11.282s), integrated its coverage into the existing mutation test, then that focused test passed (15.084s). Final suite on a fresh snapshot: 15 PASS (167.311s).
- L7: Scratch repository validation unavailable without `.git`; live repository validation passed. Cargo checks not applicable: Rust, toolchain and dependencies unchanged. No test failure suppressed; no real financial data used. Roadmap P03.06 checked only after functional proof; P03.07 remains pending.
