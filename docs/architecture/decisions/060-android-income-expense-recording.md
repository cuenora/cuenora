# ADR-060: Reviewed Android income/expense recording and durable retry

- **Status:** Accepted
- **Date:** 2026-11-08
- **Roadmap:** P11.05

## Context

F02 requires dated income/expense recording with optional accounts from Home,
Transactions and account details. Shared commands, financial validation and the
HTTP write adapter already exist. A lost response cannot establish whether the
server committed, and recreating an operation key risks recording money twice.
The account-onboarding journal is deliberately specific to account creation.

## Decision

Use one shared `RecordingSession` and one native modal form for all three entry
points. Keep the originating navigation mounted, preselect the detail account
and currency, and require explicit review before submission. Drafts remain in
memory, not Android saved instance state. Financial input uses existing exact
money/date/domain validation and the existing transaction HTTP adapter.

Create a separate typed `PendingTransaction` write-ahead journal before POST.
Android follows the established Keystore AES-GCM/AtomicFile/no-backup pattern,
with separate namespace, owner/origin AAD binding and operation type. Never
replace an unresolved command/key. Sign-in and reopening only expose recovery;
the user explicitly retries the original operation. Fresh owner/email/zone
checks and posting stay within `AuthSession.authorized`. Current account
eligibility is checked for new reviewed forms, not before idempotent replay;
the server remains authoritative. A definite validation rejection permits
correction, while ambiguous outcomes retain the journal.

Use the existing profile endpoint for explicit zone confirmation even when the
user has no accounts. Use an additive strict adapter for the existing read-only
category catalog. Do not introduce backend routes, schema changes, offline
writes, transaction history, transfer or correction flows.

## Consequences

- One presentation lifetime serializes submissions and rejects late success or
  failure publication after retirement. Disposal never deletes an unresolved
  financial operation.
- Same-owner process/session recovery can resolve the original key without
  silently posting. Local secure-storage failures fail closed before POST.
- The separate journal duplicates a small amount of established platform storage
  code rather than broadening the onboarding store and risking compatibility.
  A generic secure-journal refactor is intentionally deferred.
- Native iOS recording and its durable adapter remain separate work.
- Local tests do not establish hosted integration, physical-device/API23 runtime
  behavior, or complete accessibility certification.
