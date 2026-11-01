# ADR-039: One-shot recurrence job with single-session exclusion

- **Status:** Accepted
- **Date:** 2026-11-01
- **Roadmap:** P07.09

## Context

P07.08 generates one rule for a verified server-derived identity. A daily scheduler
has database-discovered rules, not bearer sessions. Multiple process invocations
must not overlap or leave advisory locks on pooled connections after cancellation.

## Decision

The maintainer approved a one-shot executable triggered daily externally, not an
embedded API task. Preserve the public generator and factor shared verified-owner
transaction authorization and generation, with a crate-private connection entry
for the job. Never construct a fictitious AuthenticatedIdentity or bypass owner
verification. Existing lifecycle/generation behavior must remain unchanged.

Use stable database-scoped session advisory exclusion on one dedicated connection
that also performs discovery and per-rule transactions. Mark it close-on-drop
before lock acquisition; explicitly unlock and close on completion. Holding a
lock on one connection while writing on others is rejected: loss of the lock
session must not leave generation capable of continuing independently.

Discover active verified-owner rules in bounded keyset pages with an initial
upper UUID boundary. Revalidate owner/state under locks. Commit each successful
rule independently; a failed rule rolls back and does not prevent processing the
remaining page. Aggregate failures produce a nonzero process status. No financial
or per-user diagnostic data is printed.

## Consequences and tradeoffs

- External scheduling is an operational prerequisite; job execution is independent
  of HTTP uptime, but daily timing is not each rule's precise local midnight.
- Coarse exclusion serializes job instances and bounds database pressure. A slow
  run causes overlapping daily invocations to skip, while durable generation
  progress enables later catch-up.
- Closing the dedicated session avoids pooled advisory-lock leakage; network
  failure detection may delay admission until PostgreSQL releases the old session.
- Existing HTTP operations share verified-owner primitives but not job admission;
  they retain their original per-owner serialization and transport contracts.
- No scheduler installation, migration, dependency or financial posting is added.
