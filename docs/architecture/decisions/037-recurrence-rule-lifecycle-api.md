# ADR-037: Serialized recurrence rule lifecycle API

- **Status:** Accepted
- **Date:** 2026-11-01
- **Roadmap:** P07.07

## Context

P07.02 defines preserved occurrence snapshots and civil-date pause exclusions.
P07.03 stores progress/history but does not serialize lifecycle operations.
Mutable rules make creation retries ambiguous if compared with current terms.
Calendar edits must not retroactively materialize revised slots.

## Decision

Implement owner-verified POST creation, partial PATCH and pause/resume assignments
with bounded owner → rule → account locks. Reuse existing Money, BusinessDate,
UUID and named-zone parsers. Keep transport separate from transactional persistence.
An additive permanent creation-key table stores owner/key/request hashes and rule
identity. Matching retries reconstruct the initial normalized representation,
not current mutable state, following the existing expense-creation approach.

The maintainer approved future-only start/cadence/zone edits: advance inclusive
coverage to the maximum of old/new-zone edit days and prior progress. Preserve
anchors, exclusions, state and occurrence snapshots. Term-only edits retain
progress. The maintainer also approved rejecting archived accounts on creation
or reassignment while preserving previously established references.

Pause/resume dates come from one server instant captured after locking, interpreted
in the rule zone. Union completed half-open pauses atomically; no anchor reset.
Repeated state assignments are no-ops. A civil date before the stored open pause
rejects rather than inventing a negative interval. Profile zone is not consulted.

## Consequences and tradeoffs

- Permanent retry metadata adds storage/migration cost but remains correct after
  edits, pause and account archival; current-rule comparison cannot provide this.
- Owner-level serialization matches existing financial operations and avoids
  archive/selection races, at the cost of serializing one owner's writes.
- Simple state assignments avoid a mutation-key ledger but cannot recognize a
  delayed retry after an inverse action. Clients must serialize intent.
- P07.08 must respect the inclusive cursor, accumulated exclusions and the same
  lock order. This decision does not implement generation, jobs or posting.
- Rule-zone civil labels survive zone edits; existing occurrences are never
  reinterpreted. The conservative old/new-day maximum avoids reopening dates.

See the [API contract](../../product/recurrence-rules.md) and
[execution plan](../../plans/active/2026-11-01-recurrence-rules.md).
