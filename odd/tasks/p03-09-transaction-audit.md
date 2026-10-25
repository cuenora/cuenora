# P03.09 — Transaction audit storage
Objective: minimal append-only transaction audit. Branch: `feat/p03-09-transaction-audit`; baseline: `87fc17c`.
Delivery: ask-on-risk; forecast ~340 authored changed lines, one implementation unit. Runner: isolated PostgreSQL Compose tests.

## Specs
- S1: “Comencemos con la tarea P03.09 del plan master”; master task: “Create `transaction_audit` migration.” Add storage only; no API, automatic auditing, auth, P03.10 or P05.13 implementation.
- S2: User “Sí” approved: “historial **append-only** con propietario como actor, transacción, acción (`create`, `edit`, `void`), fecha UTC y nombres de campos modificados, **sin copiar valores ni descripciones**.” Preserve this boundary with owner-scoped FK, actor equality, restricted field-name vocabulary, UTC instant bounds and immutable rows; acknowledge no historical-value reconstruction.

## Tasks
- T1 | S1-S2 | inline (one cohesive understood storage change; no writer trigger) | implemented; verification pending | migration, tests and docs; RED 4 missing-table failures, GREEN 4 groups (125.327s); Risk: items 1 and 2 (persisted audit contract and ownership links).
- T2 | S1-S2 | delegated verification (items 1 and 2), then native review if due | pending | independent probes, bounded suite, native assessment and closure; commit pending.

## Log
- L1: User: “Comencemos con la tarea P03.09 del plan master”.
- L2: User: “Sí” to the S2 proposal. Full proposal: “P03.09 crea `transaction_audit`, pero el plan no define qué conservar del cambio. Propongo un historial **append-only** con propietario como actor, transacción, acción (`create`, `edit`, `void`), fecha UTC y nombres de campos modificados, **sin copiar valores ni descripciones**. Reduce datos sensibles duplicados, pero no permite reconstruir versiones anteriores; guardar valores anteriores/nuevos sí lo permitiría. ¿Confirmás la propuesta de guardar solo los campos modificados, sin sus valores?”
- L3: Baseline main 87fc17c; only pre-existing untracked .atl/. Docker available. RDD globally on. No remote authorization. Field vocabulary follows editable persisted transaction fields; empty summaries allowed for actions without field changes. Server timestamp default does not authenticate callers or forbid privileged explicit timestamps. IDs distinguish events, not wall-clock order.
- L4: RED observed: four audit groups failed specifically with “Audit migration is missing” (36.678s); no migration existed. Added migration 0007 afterward.
- L5: GREEN: all four focused audit groups passed (125.327s). Repository validation and diff whitespace check passed. No existing helper or command implementation changed. T2 will run full suite, baseline prior-test comparison and independent isolated probes.
