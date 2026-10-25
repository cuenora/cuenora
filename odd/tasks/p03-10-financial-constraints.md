# P03.10 — Financial storage constraints
Objective: complete existing financial guards without a posting protocol. Branch: `feat/p03-10-financial-constraints`; baseline: `bee83bc`.
Delivery: ask-on-risk; forecast ~370 authored changed lines, one implementation unit. Runner: isolated PostgreSQL Compose tests.

## Specs
- S1: User: “Comienza la sigiente tarea P03.10”. Master task: “Add checks for amounts, operation kinds, supported currencies, account-type income restrictions, and user/date indexes.” Retain existing amount/kind/supported-currency checks; complete transaction/account and movement/account currency constraints, ordinary-income savings exclusion and transaction user/date index. No automatic posting or API.
- S2: User “Sí” approved: “permitir cambiar el tipo o moneda de una cuenta **solo si no invalida transacciones ni movimientos existentes**. Por ejemplo, una cuenta con ingresos ordinarios vinculados no podría convertirse en ahorro, aunque estén anulados.” Test both transaction and account updates, including concurrent writes; reject existing incompatible data without rewriting history.

## Tasks
- T1 | S1-S2 | inline (one cohesive constraint change; no writer trigger) | implemented; focused checks passed | migration, regression tests and docs; Risk: items 1, 2 and 4 (existing data, ownership links, concurrent changes); commit pending.
- T2 | S1-S2 | delegated verification (named risk items), native review if due | pending | focused/full checks, isolated independent probes and closure.

## Log
- L1: User: “Comienza la sigiente tarea P03.10”.
- L2: User “Sí” to: “Los checks de importes, operaciones y monedas admitidas ya existen. Falta validar la moneda contra cada cuenta, impedir ingresos ordinarios en cuentas de ahorro y añadir el índice usuario/fecha. Hay una política pendiente: propongo permitir cambiar el tipo o moneda de una cuenta **solo si no invalida transacciones ni movimientos existentes**. Por ejemplo, una cuenta con ingresos ordinarios vinculados no podría convertirse en ahorro, aunque estén anulados. Esto preserva coherencia, pero limita reclasificaciones. ¿Confirmás esta regla?”
- L3: Main bee83bc verified after user pull; only preexisting untracked .atl/. Preserve old migration bytes, owner FKs, exact amount bounds and snapshot semantics. Movement docs over-assign posting membership/sign/counts to P03.10; clarify these remain deferred to application posting/corrections. Migration uses native immediate FKs and validates existing rows. No remote operations authorized.
- L4: RED before migration: four focused groups failed (46.370s). Additional subtest RED observed all new source/destination/posting currency and savings create/edit/assignment rules accepting invalid writes (11 failures, 22.112s). Added migration 0008 only afterward.
- L5: Initial GREEN run exposed test expectation errors, not accepted invalid data: parent ON UPDATE RESTRICT returns23001 (child FK violation23503), and UNIQUE reapplication returns42P07. Corrected those exact assertions; no constraint weakened or migration changed.
- L6: All four focused groups now pass: currency/savings insert/edit passed in the initial migration run; the other three passed together after SQLSTATE assertion corrections (105.734s), including four observed blocking interleavings. Repository validator and diff whitespace checks pass. Full suite and independent verification pending.
