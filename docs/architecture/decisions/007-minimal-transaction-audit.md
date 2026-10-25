# ADR-007: Minimal append-only transaction audit

- **Status:** Accepted
- **Date:** 2026-10-25
- **Scope:** P03.09

## Decision

Store transaction audit events append-only with owner as actor, owned transaction,
create/edit/void action, UTC instant and changed field names only. Do not duplicate
financial values, descriptions, snapshots or free-text summaries. The maintainer
explicitly confirmed this policy before implementation.

## Rationale and consequences

The master plan's safe change summary was underspecified. Value diffs or full
snapshots could reconstruct old records, but duplicate sensitive data and expand
retention obligations. Field-name summaries minimize duplication but cannot
reconstruct previous values. Existing financial movement history remains intact;
it is not a substitute for a historical transaction snapshot.

System actors and complete version reconstruction are not introduced. Future
mutation use cases must authenticate the actor and write changes and audit events
atomically. See [the storage contract](../transaction-audit-storage.md).
