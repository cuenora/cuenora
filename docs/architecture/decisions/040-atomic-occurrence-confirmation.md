# ADR-040: Atomic actual-payment confirmation

- **Status:** Accepted; own and independent verification passed locally
- **Date:** 2026-11-01
- **Roadmap:** P07.10

## Decision and rationale

Compose existing ordinary-expense validation and posting with the occurrence
transition inside one transaction. Do not call an expense API that commits before
linking the occurrence. Serialize on the verified owner, then the occurrence and
actual account, following existing owner-first financial writes. Rule state and
planned terms do not override an actual payment review.

Use an additive permanent confirmation-key table with an independent owner-wide
namespace. Bind owner, occurrence and expense through a composite foreign key to
the confirmed occurrence link; store key/request hashes, not value snapshots.
Keep existing ordinary-expense retry hashes and responses unchanged. New keys
cannot reconfirm terminal occurrences; matching keys reconstruct their initial
normalized response even after later expense corrections or voiding.

## Tradeoffs

A dedicated table and redundant unique target constraint cost schema surface but
make confirmation identity explicit and enforceable without overloading ordinary
expense keys or rewriting occurrence history. Reusing a hash derived from an
occurrence and an ordinary expense key would hide the operation namespace and
make owner-wide key-conflict semantics less explicit.

Owner serialization is deliberately coarser than per-occurrence locking. It
matches existing financial writes and avoids account/archive/profile races;
independent owners still proceed concurrently. Late errors and cancellation rely
on database transaction rollback rather than compensation after partial commits.

See the [API contract](../../product/recurrence-confirmation.md) and
[execution record](../../plans/active/2026-11-01-confirm-occurrence.md). This adds
no payment execution, automatic confirmation, skip/postpone or mobile runtime.
