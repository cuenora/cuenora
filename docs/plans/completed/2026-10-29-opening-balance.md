# Execution plan: P05.03 dated opening balances

- **Status:** Completed; merged through PR #72
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.03; confirmed-zone/date runtime prerequisite from P03.03
- **Tracking issue / PR:** [PR #72](https://github.com/cuenora/cuenora/pull/72)
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
- [x] Own seam suite, independent high-risk verification and applicable native review.

## Acceptance criteria

- [x] Past/today positive and negative openings post exactly once for both account types.
- [x] Zero/omitted openings have no financial event/movement/audit effects.
- [x] Invalid/future dates, unknown/unconfirmed zones and unauthorized identities write nothing.
- [x] Failed late writes roll back account/event/movement/audit/key together.
- [x] Retries, conflicting content, old keys and concurrent profile/creation calls are safe.
- [x] Existing auth and non-opening creation behavior remains unchanged.

## Verification

| Check | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Domain/profile/opening focused tests | Observed RED then GREEN | Dates5, profile ordinary/parser/DB3, openingDB7 passed after observed RED | Pass |
| Repository, fmt, Clippy, Cargo | Clean and all ordinary tests pass | Repository/fmt/Clippy/Cargo passed | Pass |
| Disposable PostgreSQL harness | Real isolated persistence/concurrency proof | All51infrastructure cases passed (1009.527s), including populated-key upgrade | Pass |
| Independent verifier | PASS per S1–S4 | Not run | Pending |
| Native assessment/review | Follow user-owned switch and exact continuations | High tier; consent granted; four lenses approved without findings; exact acknowledgement burned authority | Pass |

## Risks, assumptions, and decisions

User explicitly approved only the minimal profile prerequisite in addition to
P05.03. Risk: items 1–5 (persistent idempotency extension, authorization, shared
creation behavior, concurrency and migration/dependencies). The maintainer explicitly approved a new single-PR size exception for this
feature (approximately1,950authored lines). No remote publication authorization.

## Completion notes

Implemented in `6804f91`; own/independent proof passed and native four-lens
review approved without findings. Exact acknowledgement completed for lineage
`review-b6759fa82168d894`. P05.03 is verified and merged through PR #72.
A working zone-confirmation API does not implement the missing email-verification
journey or make the product production-ready.

### Verification evidence and remaining decisions

Full Cargo suite: 86 passed, 56 PostgreSQL cases ignored before retaining the
independent regression probe. Full infrastructure: all 51 cases passed. The
independent verifier passed S1–S4, ran profile/opening/migration targets, retained
account compatibility tests and compared 25 baseline/current HTTP captures.
The retained probe increases opening cases to eight. Optional cargo-audit was
unavailable (not installed); no dependency vulnerability clearance is claimed.
No source correction was required. The new single-PR size exception is approved.
Native review was approved and acknowledged; publication subsequently received
explicit authorization, and PR #72 merged with all four hosted checks successful. Final focused rerun passed eight opening cases; Cargo86passed/57DBignored and
Clippy passed. Full51infrastructure cases were not rerun after this test-only
probe addition; production code was unchanged. See the task log.

Publication and merge are confirmed; this record is archived under `completed/`. The passive closure changes only Markdown; no executable or contract
bytes change after review.

### Confirmed integration

PR #72 merged at `6c2e93a`; its full tree matched published head `9feb7fb`.
Repository validation, Rust, dependency review and dependency audit all succeeded
in hosted CI. Local `main` was fast-forwarded after explicit maintainer approval.
