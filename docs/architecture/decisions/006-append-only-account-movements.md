# ADR-006: Preserve account postings with compensating entries

- **Status:** Accepted
- **Date:** 2026-10-25
- **Scope:** P03.08 storage; correction workflows remain P05.13

## Context

Account movements are the source of truth for balances. Overwriting or deleting
original postings would lose the history explaining those balances. The user
approved preserving originals and correcting through compensating movements.

## Decision

Keep movement rows append-only, enforced for ordinary SQL UPDATE/DELETE/TRUNCATE.
Corrections append compensating postings rather than modifying originals. Every
posting references its owned transaction and account. Multiple postings may link
the same event/account, so compensation is not blocked by a uniqueness rule.

## Consequences

History remains inspectable at the cost of more rows and explicit correction
orchestration. Balance queries must include compensation entries, not filter
originals by an event's current state. Mutable postings were rejected because
simpler updates would sacrifice this traceability. Privileged schema maintenance
and privacy erasure need separate, reviewed procedures. P05.13 must define atomic
correction behavior and detailed reversal linkage before exposing it to clients.
