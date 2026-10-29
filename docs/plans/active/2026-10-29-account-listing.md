# Execution plan: P05.04 account listing

- **Status:** Implemented and verified locally in `9a03322`; native review acknowledged; unpublished
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.04; operation-specific P04.06/P04.10 ownership proof
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Expose `GET /v1/accounts` to an authenticated, email-verified owner. Return active
and archived metadata only, with bounded cursor pagination and cross-user isolation.
The approved requirements and evidence live in the [task log](../../../odd/tasks/p05-04-account-listing.md).

## Existing state and evidence

P05.02/P05.03 are merged. Account POST/opening creation, bearer identity, typed
errors and metadata storage exist. At the baseline, account GET, pagination and read authorization
were not implemented. The owner index and creation/status fields already exist.

## In scope

- Verified-owner-only listing with the existing five metadata fields.
- Active and archived accounts; default50/max100 cursor pages, created_at/id ascending.
- Stable bounded reads, generic errors, no-write guarantees and isolation tests.
- OpenAPI, API documentation, setup and execution evidence.

## Out of scope

Balances (P05.05), listing other resources, archive/filter/mutation workflows,
email verification delivery/recovery, mobile screens and remote publication.

## Constraints and invariants

Reuse bearer identity, error/request-ID/no-store handling and existing storage.
Every account/cursor read includes authenticated ownership; client IDs never grant
access. An undated metadata read needs no profile zone. Preserve existing POST
and opening behavior, stored names and financial/audit records. No new migration
or library is expected; an Axum query feature may be enabled if needed.

## Steps and acceptance

- [x] Observe meaningful RED against the existing authenticated GET behavior.
- [x] Implement bounded verified-owner reads, strict query input and metadata response.
- [x] Prove ownership, pagination/ties/limits, active/archive status, empty results and no writes.
- [x] Run own relevant seam suite and independent high-risk verification.
- [x] Follow applicable native review and record the final candidate and outcome.

## Verification

| Check | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Listing HTTP/DB targets | RED then GREEN for every approved rule | Verified GET405vs200 observed; listing5DB passed | Pass |
| Repository, fmt, Clippy, full Cargo | Clean, unchanged existing tests pass | Repository/fmt/Cargo pass; helper-only Clippy correction then recheck passed | Pass |
| Disposable account listing/create/opening suite | Owner/permission and POST compatibility proof | Listing5/create7/opening8 all passed; retained probe target6 then passed | Pass |
| Independent verifier | PASS S1–S4 and baseline differential | PASS S1–S4; 55 identical normalized HTTP cases, coherent concurrent snapshot probe | Pass |
| Native assessment/review | Exact user-owned lifecycle if due | Four immutable-patch lenses: no findings; exact acknowledgement burned authority | Pass |

## Risks and decisions

Risk: items 2/3/4 (ownership, shared route compatibility, consistent read ordering),
and item 5 (query-feature dependency wiring). User approved the
listed initial contract and a new single-PR size exception (~1,300changed lines),
not remote publication. No migration,
balance computation or silent archive filtering is authorized.

## Completion notes

Own and independent proof passed. P05.04 is locally complete in `9a03322`; all four native lenses returned no findings and exact acknowledgement burned authority. The single-PR size exception is approved; publication remains unauthorized. No runtime correction was required. Seeded
verification tests the financial gate, not the unimplemented email user journey.

Full Cargo passed (86 ordinary/doctests; 62 PostgreSQL cases ignored before the
retained probe, then 63). A test-only high-arity helper Clippy failure was corrected
with AccountFixture, without suppressions or runtime changes; rechecks passed.
The retained six-case listing target passed after the independent probe was added.
No full unchanged migration sweep was run because no schema changes were made;
scoped listing/create/opening coverage is recorded above. Optional cargo-audit
was unavailable (not installed); no vulnerability clearance is claimed.

Native review: `review-b8e262b117a29da8`, target
`sha256:32bee4271188ba6c27fb04481e346ef522a884274495fa7b31d34b7ee21d3e12`.
Reviewers inspected immutable patches; they did not rerun tests. No native
correction was needed. Keep this record active until an authorized PR is merged.
