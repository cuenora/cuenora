# ADR-038: Atomic due-occurrence generation

- **Status:** Accepted
- **Date:** 2026-11-01
- **Roadmap:** P07.08

## Context

Rule operations already serialize owner → rule and preserve inclusive coverage
and half-open civil pause exclusions. Generation must recover active backlog
without rewriting occurrences or introducing premature financial accounting.
The maintainer approved generation only through rule-local today inclusive.

## Decision

Add a callable backend application operation with a server-derived identity and
trusted clock closure evaluated after owner/rule locks. Reuse existing verified
owner admission, calendar arithmetic and local-date conversion. Do not add an
HTTP route or worker. Snapshot the locked rule into absent eligible slots using
one set-based INSERT and PostgreSQL-native exclusion checks; unique-slot conflicts
do nothing. Commit inserted planned rows and monotonic coverage atomically.

Cap coverage at the inclusive rule end as well as today, so a later end extension
can recover newly eligible active backlog. Keep existing calendar-edit coverage
even when the clock or end moves backward. Paused and already-covered runs do not
change data or audit times. Existing slots remain untouched in every state.

## Consequences and tradeoffs

- Owner-level locks match lifecycle operations, trading per-owner write throughput
  for consistent snapshots and no edit/pause races.
- A trusted injected clock enables deterministic zone/midnight tests but must not
  become client input when the future worker is wired.
- Set-based insertion avoids per-slot round trips. Calendar enumeration uses a
  date-range-bounded vector; statement timeouts can reject an unusually large
  backlog atomically. Job admission and operational retry remain P07.09.
- No new migration, dependency, retry ledger or financial posting is required.

See the [generation contract](../../product/recurrence-generation.md).
