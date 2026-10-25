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
- [x] 3. Run suite, independent verification and native review when consented.

## Acceptance criteria

- [x] Owner-scoped links, actor equality and restricted summaries are tested.
- [x] Rejected writes and append-only guards preserve financial and audit data.
- [x] Fresh schema, index, migration reapplication and DDL rollback are tested.
- [x] Documentation distinguishes storage from future auditing workflows.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Four `PostgresComposeTest.test_audit_*` groups | RED, then GREEN | 4 missing-table failures; then 4 groups passed (125.327s) | Pass |
| `python3 tests/infrastructure/test_postgres_compose.py` | Pass | 29 tests passed (557.251s); baseline 25 passed (424.598s) | Pass |
| `python3 scripts/check_repository.py` | Pass | Repository validation passed | Pass |
| Independent isolated PostgreSQL probes | S1/S2 pass | 72 assertions, 29 atomic rejections, 7 unchanged baseline SQL results | Pass |
| Native assessment/review | Follow provider transitions | High assessment; user granted; four lenses, no findings; exact acknowledgement burned authority | Pass |
| Rust 1.99.1 `cargo fmt --all -- --check` | Pass | Passed | Pass |
| Rust 1.99.1 `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pass | Passed | Pass |
| Rust 1.99.1 `cargo test --workspace --all-features --locked` | Pass | 21 tests passed | Pass |

## Risks, assumptions, and decisions

Risk: items 1 and 2 (persisted audit structure and owner/actor relationships).
Empty field lists are valid, including create/void events. Names describe changes,
not historical values; event truthfulness and atomic recording belong to future
application use cases. No extra audit retention/erasure policy is introduced.

## Completion notes

Implementation `5a6f1ac` and S1/S2 are locally verified. Native lineage
`review-9b39a05ff9e5a65f` was acknowledged with authority burned; no correction
was required. All checks above passed; no checks were skipped. No push or PR
was requested. Keep this plan active until the implementation PR is merged.
P03.10–P03.12 and automatic application auditing remain pending.
