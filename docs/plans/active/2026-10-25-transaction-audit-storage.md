# Execution plan: Transaction audit storage

- **Status:** Active
- **Date:** 2026-10-25
- **Roadmap task ID(s):** P03.09
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Create minimal append-only audit storage without duplicating financial values.
No API or automatic audit generation is introduced.

## Existing state and evidence

Transactions and append-only movements exist through migration 0006. The master
plan requires an actor, action, record, timestamp and safe change summary.
The maintainer approved field names only, not values or descriptions.
See [the task log](../../../odd/tasks/p03-09-transaction-audit.md).

## In scope

- Migration 0007, owner/actor constraints, immutable audit events and indexes.
- Isolated PostgreSQL tests and the storage contract.

## Out of scope

- Automatic auditing, authentication, mutation APIs, P03.10 and P05.13.
- Snapshots, value diffs, free-text summaries and system actors.

## Constraints and invariants

Owner is the actor. Audit references an existing transaction of that owner.
Only create/edit/void actions and known field names are stored. Rows cannot be
updated, deleted or truncated through ordinary DML. Timestamps are instants, not
ordering or idempotency identifiers. Privileged database administration remains
outside these guards; future use cases must enforce authenticated ownership.

## Steps

- [x] 1. Observe missing audit storage in focused tests (RED).
- [x] 2. Add migration and verify focused tests (GREEN).
- [ ] 3. Run suite, independent verification and native review when consented.

## Acceptance criteria

- [ ] Owner-scoped links, actor equality and restricted summaries are tested.
- [ ] Rejected writes and append-only guards preserve financial and audit data.
- [ ] Fresh schema, index, migration reapplication and DDL rollback are tested.
- [ ] Documentation distinguishes storage from future auditing workflows.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Four `PostgresComposeTest.test_audit_*` groups | RED, then GREEN | 4 missing-table failures; then 4 groups passed (125.327s) | Pass |
| `python3 tests/infrastructure/test_postgres_compose.py` | Pass | Pending | Pending |
| `python3 scripts/check_repository.py` | Pass | Repository validation passed | Pass |
| Independent isolated PostgreSQL probes | S1/S2 pass | Pending | Pending |
| Native assessment/review | Follow provider transitions | Pending | Pending |

## Risks, assumptions, and decisions

Risk: items 1 and 2 (persisted audit structure and owner/actor relationships).
Empty field lists are valid, including create/void events. Names describe changes,
not historical values; event truthfulness and atomic recording belong to future
application use cases. No extra audit retention/erasure policy is introduced.

## Completion notes

Pending verification. Keep this plan active until the implementation PR is merged.
