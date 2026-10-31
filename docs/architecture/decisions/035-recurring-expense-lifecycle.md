# ADR-035: Separate recurring rules, occurrences and posted expenses

- **Status:** Accepted; specification only, runtime pending
- **Date:** 2026-10-31
- **Roadmap:** P07.02

## Context

A recurrence plans a payment but does not prove that money was spent. Rules may
be paused or edited after occurrences already exist. Rewriting those occurrences
would silently change reviewed commitments; recreating missed paused slots on
resume would introduce unexpected obligations.

## Decision

Model a rule separately from its occurrences and from confirmed expense events.
Rules are active or paused; occurrences are planned, confirmed or skipped.
Overdue and schedule exhaustion are derived conditions, not automatic posting.

Pausing retains all existing occurrences. Resuming follows the original calendar
without backfilling ungenerated slots from paused intervals. Rule edits affect
only new occurrences; changing existing planned terms requires an explicit action.
Keep original scheduled-slot identity separate from an effective postponed due
date, so postponement cannot cause generation of a duplicate occurrence.

Confirmation atomically links one expense to the occurrence and removes that
occurrence from pending commitments. It must reuse financial posting guards and
idempotency guarantees. Rule changes never rewrite confirmed financial events.

The [model, transitions and acceptance scenarios](../../product/recurrence-lifecycle.md)
are the normative specification. No persistence or API contract is implemented here.

## Tradeoffs

Preserving generated terms makes commitments stable but a rule edit does not
update all pending payments in bulk. Explicit occurrence actions are required.
Retaining pause history and scheduled-slot identity adds generation metadata,
but avoids backfill surprises and duplicate postings. Upcoming commitments and
realized expenses remain separate even when a rule is paused or exhausted.
