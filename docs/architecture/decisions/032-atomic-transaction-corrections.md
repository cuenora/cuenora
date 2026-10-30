# ADR-032: Atomic corrections with linked inverse postings and permanent retries

- **Status:** Accepted; own/independent verification passed; native review pending
- **Date:** 2026-10-30
- **Scope:** P05.13

## Decision

Expose owner-verified partial edits and voids for all five recorded event kinds.
Keep kind immutable to preserve the operation's meaning and savings guards.
Apply corrections under the existing owner-first serialization boundary. In one
SQL transaction, append linked inverses and replacements, update event metadata,
append the actual changed field names to minimal audit, and persist a permanent
mutation-key fingerprint. Void compensates without deleting or restoring history.

The maintainer explicitly included openings, transfers and credited interest,
and approved requiring reactivation for archived-account movement corrections,
while permitting metadata-only corrections. Full behavior is in the
[correction contract](../../product/transaction-corrections.md).

## Rationale and tradeoffs

State assignment alone is not sufficient for exactly-once retries: an old PATCH
retried after an intervening edit could revert the newer amount. A permanent
owner-scoped key binds target/action/normalized patch and returns only the
original ID/state acknowledgement. It stores no sensitive value snapshots.
Creation keys remain separate and unchanged. Storage grows without expiry;
clients need a fresh key for each deliberate correction.

Migration 0014 adds nullable reversal linkage to existing postings and an
immutable retry table. Existing rows stay intact. Composite owner/event foreign
keys and a unique reversal reference prevent cross-event links and duplicate
inverse references. The application verifies effective posting sets and exact
inverse account/amount/currency/date before appending. This is not a database
trigger that automatically posts or validates every arbitrary SQL write.

Inverses use the original economic date; replacements use the corrected date.
This repairs historical periods without pretending the audit instant is the
business date. Original movement values remain traceable under ADR-006; audit
still contains field names only under ADR-007. No full transaction snapshots,
FX rate precision, optimistic versioning or restoration lifecycle is introduced.

## Verification and recovery

Public HTTP tests run against isolated real PostgreSQL, including both transfer
legs, all kinds, reassignment, accountless corrections, archives, retries,
concurrent mutations and late rollback. Baseline creation/read behavior must
remain unchanged. Apply migrations transactionally in order; populated data
requires controlled forward repair, not deletion or rewriting original movements.
