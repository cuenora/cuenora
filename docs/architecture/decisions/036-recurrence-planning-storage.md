# ADR-036: Additive recurrence planning storage

- **Status:** Accepted; local migration and verification complete
- **Date:** 2026-10-31
- **Roadmap:** P07.03

## Decision and rationale

Store rules and occurrences separately from posted transactions. Use composite
owner/currency/category foreign keys, immutable scheduled-slot identities and a
unique owner-matched expense link. Native references protect reverse parent-key
changes without adding check-then-write races. Terminal occurrence history and
recurrence identities cannot be rewritten or deleted.

Keep completed pause exclusions in PostgreSQL's native `DATEMULTIRANGE`, an open
pause in `paused_on`, and optional generation coverage in `generated_through`.
Canonical union retains excluded civil dates without a third table, an extension
or unvalidated JSON. The original calendar anchor remains `starts_on`; its day is
generated independently of shortened months. P07.07/P07.08 still own atomic
progress maintenance, edit boundaries, locking and generation algorithms.

## Tradeoffs and verification

Multiranges couple this representation to PostgreSQL, already the chosen storage,
and require future adapters to map dates explicitly. Identifier-shape checks do
not replace application IANA-zone validation. Planning snapshots may differ from
actual payment terms; only explicit confirmation may create and link the expense.

The [storage boundary](../recurrence-storage.md) and
[execution record](../../plans/active/2026-10-31-recurrence-storage.md) document
real isolated-database proof. No recurrence runtime/API or remote delivery is
part of this migration.
