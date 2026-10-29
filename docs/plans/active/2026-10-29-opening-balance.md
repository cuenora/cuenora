# Execution plan: P05.03 dated opening balances

- **Status:** Implemented and verified locally; native review pending; single-PR size exception approved
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.03; confirmed-zone/date runtime prerequisite from P03.03
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Accept an optional dated opening balance during account creation. Post nonzero
values as a special event and append-only movement, never ordinary income, in the
same transaction as the account, minimal audit and lifetime idempotency record.
Include the explicitly approved minimal profile-zone confirmation operation.
The full specs and task evidence live in [the task log](../../../odd/tasks/p05-03-opening-balance.md).

## Existing state and evidence

P05.02 is merged at `137e2e9`: account creation validates domain metadata,
requires verified email and permanently deduplicates per owner/key. Opening event,
movement and audit tables already exist, but there is no date/profile-zone runtime.

## In scope

- Pure strict business-date validation with deterministic tests.
- Authenticated explicit profile IANA-zone confirmation, not general profile CRUD.
- Optional exact opening input; confirmed-zone/nonfuture validation; atomic
  account/event/movement/create-audit/key persistence and compatible safe replay.
- Forward migration, OpenAPI, tests and documentation for these behaviors.

## Out of scope

Email verification/recovery runtime, mobile screens, listing/balance endpoints,
transactions other than opening, transfers and corrections. No verification bypass.

## Constraints and invariants

Reuse Money, AccountName, Currency, bearer identity and the existing owner-row
serialization protocol. Preserve P05.02 responses and old idempotency records.
No financial floating point; no ordinary income/category; no UPDATE/DELETE to
existing postings/audit. One trusted UTC instant per operation supplies user today;
domain rules do not access the clock. Rejected requests cannot mutate state.

## Steps

- [x] Strict domain dates and deterministic boundary cases.
- [x] Minimal profile-zone confirmation with ownership/no-write tests.
- [x] Atomic opening posting, old/new idempotency tests and contracts.
- [ ] Own seam suite, independent high-risk verification and applicable native review.

## Acceptance criteria

- [ ] Past/today positive and negative openings post exactly once for both account types.
- [ ] Zero/omitted openings have no financial event/movement/audit effects.
- [ ] Invalid/future dates, unknown/unconfirmed zones and unauthorized identities write nothing.
- [ ] Failed late writes roll back account/event/movement/audit/key together.
- [ ] Retries, conflicting content, old keys and concurrent profile/creation calls are safe.
- [ ] Existing auth and non-opening creation behavior remains unchanged.

## Verification

| Check | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Domain/profile/opening focused tests | Observed RED then GREEN | Dates5, profile ordinary/parser/DB3, openingDB7 passed after observed RED | Pass |
| Repository, fmt, Clippy, Cargo | Clean and all ordinary tests pass | Repository/fmt/Clippy/Cargo passed | Pass |
| Disposable PostgreSQL harness | Real isolated persistence/concurrency proof | All51infrastructure cases passed (1009.527s), including populated-key upgrade | Pass |
| Independent verifier | PASS per S1–S4 | Not run | Pending |
| Native assessment/review | Follow user-owned switch and exact continuations | Not run | Pending |

## Risks, assumptions, and decisions

User explicitly approved only the minimal profile prerequisite in addition to
P05.03. Risk: items 1–5 (persistent idempotency extension, authorization, shared
creation behavior, concurrency and migration/dependencies). The maintainer explicitly approved a new single-PR size exception for this
feature (approximately1,950authored lines). No remote publication authorization.

## Completion notes

Implementation and own/independent proof complete. Keep the P05.03 checkbox
open until final review/delivery decisions are resolved.
A working zone-confirmation API does not implement the missing email-verification
journey or make the product production-ready.

### Verification evidence and remaining decisions

Full Cargo suite: 86 passed, 56 PostgreSQL cases ignored before retaining the
independent regression probe. Full infrastructure: all 51 cases passed. The
independent verifier passed S1–S4, ran profile/opening/migration targets, retained
account compatibility tests and compared 25 baseline/current HTTP captures.
The retained probe increases opening cases to eight. Optional cargo-audit was
unavailable (not installed); no dependency vulnerability clearance is claimed.
No source correction was required. The new single-PR size exception is approved. Native review remains pending
after the local implementation commit. Final focused rerun passed eight opening cases; Cargo86passed/57DBignored and
Clippy passed. Full51infrastructure cases were not rerun after this test-only
probe addition; production code was unchanged. See the task log.
