# ADR-054: Explicit shared account and transaction repositories

- **Status:** Accepted
- **Date:** 2026-11-07
- **Related:** P10.08, ADR-050, ADR-051, ADR-052, ADR-053

## Context

Native workflows need typed resource operations without JSON or platform storage
in the financial domain. The maintainer explicitly included existing read and
write operations in P10.08. Transport and secure storage do not yet orchestrate
authentication. Server account metadata intentionally omits the owner ID required
by the shared domain model.

## Decision

Expose account and transaction repository interfaces, typed commands/queries and
HTTP implementations over the existing caller-owned `ApiClient`. Keep domain
models unchanged. Use explicit immutable `RepositoryContext` with caller-supplied
owner identity and a redacted bearer credential. The identity completes domain
mapping; it is not evidence of authorization. Instantiate new repositories on
session/user change. Do not decode tokens, read SessionStore, refresh, retry,
generate mutation keys or make requests during construction.

Account operations list/create/update; status updates archive/reactivate.
Transaction operations list/create/edit/void; creation covers income, expense,
interest and transfer. Opening balances are created only through account creation.
There is no hard delete or restore endpoint. Caller-owned idempotency keys apply
only where the existing API requires them; preserve the same key for an explicitly
retried logical operation. Server ownership, dates, financial eligibility and
atomic postings remain authoritative.

Map wire UUIDs/dates and exact decimal amount strings through shared adapters.
Keep listed account balances outside account metadata. Preserve both actual
transfer amounts and informational quote comparisons without computing an FX
replacement. Edit/void return acknowledgements, not invented snapshots. Nullable
PATCH fields explicitly distinguish absent, null and a supplied value.
PATCH amount fields use currencyless `DecimalAmount` text with exact recorded
scale/range validation, because currency is a separate optional field. Never
accept `Money` and silently discard its currency. Creation/recorded amounts
continue to use `Money`. Creation responses must match supplied event fields;
mutation acknowledgements must match their target and action. JSON boolean
fields require actual boolean tokens, not quoted boolean text.

`RepositoryResult` maps transport failures unchanged. Unexpected success status,
missing/wrong field types and invalid domain values become redacted
`ApiFailure.InvalidResponse` with status/request ID, not raw payload diagnostics.
Coroutine cancellation propagates. Unknown extra resource fields do not require
a client release; unknown required enum values are invalid responses.

Common test fakes implement the same interfaces with configurable outcomes and
call capture. They do not pretend to enforce ownership, implement balances or
reproduce server idempotency; MockEngine tests pin the actual wire behavior.

## Consequences

Both targets can test repository consumers without HTTP. Test fakes live in
commonTest, not the production framework export. There is no local cache or
automatic account/transaction state reconciliation; callers fetch authoritative
data after mutations. Repository instances retain a session snapshot until
discarded, so future auth orchestration must own replacement on session changes.

Verification uses synthetic data, MockEngine and both shared targets, plus native
artifact builds. It does not prove live backend authentication/networking or UI
integration. Cross-language canonical fixtures remain P10.11.
