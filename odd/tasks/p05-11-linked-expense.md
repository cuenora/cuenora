# P05.11 — Linked expense
Record expenses against owned active accounts. Branch: `feat/p05-11-linked-expense`; baseline: `07bd9d2`.
Delivery: ask-on-risk; forecast 380 authored lines; runners: pinned Cargo and isolated PostgreSQL Compose.

## Specs

- S1: "Comencemos la tarea P05.11 del plan maestro" — "P05.11 Record an expense linked to an account." Add optional account linkage with one negative movement, atomic event/audit/key, exact currency, verified ownership, confirmed-zone/date gates and active-account enforcement. Existing approved policies permit savings expenses and negative balances.
- S2: Preserve existing accountless expense and income behavior and durable retries. Linked expense retries include normalized account identity, conflict on changed linkage and never repeat balance effects; replay precedes mutable account-state validation.

## Tasks

- T1 — S1/S2 — inline implementation (one coherent behavior; no writer trigger), RED observed (2 linked failures, 5 existing passes); GREEN observed (7 expense and 11 income PostgreSQL cases); docs updated; commit pending.
- T2 — S1/S2 — delegated independent verification, risk items 2/3/4 (ownership, shared transport/contract, posting serialization); S1/S2 passed independent isolated differential/durable-key probes; commit pending.

## Log

- L1: User request: "Comencemos la tarea P05.11 del plan maestro"
- L2: Read-only explorer mapped existing schema and policy; parent spot-checked expense/income creation and shared HTTP transport. No migration needed. Preserve `.atl/`; no remote access/publication authorized. Existing expense keys remain the expense namespace; preserve every accountless fingerprint. Owner→account lock order follows existing income/account management.

- L3: RED PostgreSQL run failed exactly the two new linked cases (400 instead of 201/404), with five existing expense cases passing. GREEN: expense seven and income eleven cases passed (31.062s). No migration. Own Cargo checks in progress; independent verification pending.

- L4: Own fmt/Clippy and Cargo workspace passed (90 passed, 92 ignored); repository/diff checks passed. Independent verifier passed S1/S2, baseline durable keys and eight invalid-input differential cases; no blockers. Parent repeated diff check and pinned the fixed P05.10 expense request hash; final focused rerun passed (7 expense cases, 15.397s); fmt/Clippy/workspace repeated successfully. No full infrastructure rerun (no schema change).
