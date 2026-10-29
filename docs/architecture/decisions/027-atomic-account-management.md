# ADR-027: Atomic owner-scoped account metadata transitions

- **Status:** Accepted; own/independent proof passed; native review pending
- **Date:** 2026-10-29
- **Roadmap:** P05.06, operation-specific P04.06/P04.10

## Decision

The maintainer approved archive blocking new income/expense/transfer postings,
zero balance as an archive prerequisite, and reversible reactivation. Preserve
read visibility/history; corrections to historical events remain separately
undecided. Renaming uses AccountName without introducing an active-only metadata
restriction. Expose name/status assignments through PATCH /v1/accounts/{id}; keep
current account IDs/owners/types/currencies and every financial record unchanged.

Require current verified email and bearer-derived ownership. Lock the owner then
account and perform the exact zero-sum check and metadata update atomically.
READ COMMITTED is required so a sum after a lock wait sees the preceding writer's
committed movements, rather than an old repeatable-read snapshot. PostgreSQL's
numeric SUM(bigint) compares to zero directly; do not narrow an overflowing total
or filter by current event state. Future postings to existing accounts must lock
in the same order and reject archived state before writing. This is a contract
for later posting implementations, not a claim that those endpoints exist or
that direct database access is an authenticated account API.

Use existing archive/updated timestamps with server time; no migration or stored
balance. Same-value assignments preserve timestamps. Update only supplied fields
against the locked current row, avoiding lost independent metadata changes.
Foreign/missing resources share the existing generic 404. Bound waits and JSON;
fail safely without partial metadata updates. No profile-zone dependency applies
to this undated operation.

## Tradeoffs and compatibility

Owner locking serializes account management with existing creation/profile
updates, at the cost of concurrency within one owner. It simplifies verification
and lock ordering without global locks. Account row locking coordinates future
posting writers; no new partial balance cache or archive trigger is introduced.

Idempotent assignments avoid storing command keys for metadata. They do not give
lifetime replay or prevent stale clients overwriting an intervening same-field
edit. A future optimistic concurrency requirement needs an explicit contract.

Lifetime creation keys must remain original-request snapshots, never current
metadata. Existing POST replay still returns its original name/active state and
never reopens an archived account. GET listing/balance responses stay unchanged.
No new audit value snapshots, financial posting, archive filter, deletion or
historical-correction policy is authorized by this task.
