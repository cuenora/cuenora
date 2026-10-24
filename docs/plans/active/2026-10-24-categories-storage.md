# Execution plan: Global built-in categories

- **Status:** Active
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.06
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Add the approved shared, read-only category catalog through a reproducible migration. No category HTTP endpoint is introduced.

## Existing state and evidence

`0001`–`0003` provide identity and account storage. The isolated Compose runner applies each SQL file transactionally. The master plan specifies categories but no initial taxonomy; the user approved the global model and exact labels recorded in the [task log](../../../odd/tasks/p03-06-categories-storage.md).

## In scope

- One categories migration, ten built-in rows, stable identifiers and write protection.
- Isolated PostgreSQL regression tests, storage documentation and roadmap status after proof.

## Out of scope

Custom categories, user provisioning, category APIs, transactions, recurrence and authentication.

## Constraints and invariants

No financial data or prior schema changes; no dependency or CI changes. Built-ins are global, not user-owned. Rejected writes and failed migrations must leave prior data unchanged. Existing identity/account cases must still pass.

## Steps

- [x] 1. Observe focused RED; implement migration and GREEN.
- [x] 2. Run suite and independent isolated verification; update documentation.
- [ ] 3. Commit the coherent unit and run native assessment; request review consent only if required.

## Acceptance criteria

- [x] Fresh databases have exactly the approved income/expense labels with stable IDs.
- [x] Global reads work and ordinary mutations are rejected without changing data.
- [x] Reapply and late migration failure roll back safely; existing tests pass.
- [x] Storage and future application responsibilities are documented.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused category tests | Fail before migration, pass afterward | 4 missing-migration failures (22.394s), then 4 pass (49.198s) | Pass |
| `python3 tests/infrastructure/test_postgres_compose.py` | Existing and new cases pass | Baseline 11 pass (118.963s); final candidate 15 pass (167.311s) | Pass |
| `python3 scripts/check_repository.py` | Repository checks pass | Passed; 116 Git-visible files | Pass |
| Independent scratch probes and parent spot check | S1–S3 satisfied | No blockers; role probe pass (11.661s), parent repeat pass (11.282s); integrated regression pass (15.084s) | Pass |

## Risks, assumptions, and decisions

Risk: items 1 (migration) and 2 (write protection). Read-only means normal DML/TRUNCATE is guarded, not that an administrator cannot alter the schema. Future catalog maintenance requires a controlled migration. No production role or authentication is introduced. Forecast ~340 authored changed lines; delivery strategy `ask-on-risk`.

## Completion notes

Implemented and verified locally. Non-owner roles share the catalog and cannot mutate it even when granted write privileges; rejected commands also roll back preceding account changes. All 26 prior Python function ASTs remain unchanged. Cargo checks were not applicable (no Rust/dependency changes); repository checks ran in the live Git checkout because extracted scratch copies lack `.git`.

Native assessment and any candidate consent remain pending. Rollback boundary is the new category migration/tests/docs only; prior migrations and existing financial records are unchanged. Keep this plan active until the corresponding PR is merged. No remote publication is authorized by this implementation request.
