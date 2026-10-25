# ADR-008: Account reference compatibility during changes

- **Status:** Accepted
- **Date:** 2026-10-25
- **Scope:** P03.10

## Decision

Allow account type/currency changes only when they preserve every existing
transaction and movement reference. The maintainer explicitly approved this rule.
An account with linked ordinary income cannot become savings, even if that income
is voided. Do not reinterpret or rewrite history to permit a change.

## Rationale and consequences

Free changes could invalidate currencies and the approved savings-income rule.
Freezing every account with any history would prohibit otherwise compatible
reclassifications. Conditional compatibility preserves references with fewer
restrictions, but limits later reclassification of ordinary-income accounts.

Use native composite FKs and generated income classification rather than
unsynchronized lookup triggers. This also protects concurrent account changes.
Migration validation rejects incompatible legacy records atomically; repair needs
separate review. Posting snapshots and future application workflows remain distinct.
See [financial storage constraints](../financial-constraints.md).
