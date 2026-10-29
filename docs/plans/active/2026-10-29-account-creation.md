# Execution plan: P05.02 account creation

- **Status:** Locally completed; publication pending
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.02 (with P04.06/P04.10 operation-specific ownership checks)
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Create an account through `POST /v1/accounts` using authenticated ownership,
verified email and mandatory lifetime idempotency. All existing HTTP endpoints
move to `/v1` together; old routes have no aliases. The full specification,
work units and evidence live in [the task log](../../../odd/tasks/p05-02-account-creation.md).

## Existing state and evidence

Account metadata and storage exist. API routes currently expose only auth and
health; verified-email delivery and confirmation are design-only. Account names
may duplicate, so retries require a separate owner-scoped idempotency record.

## In scope

- Uniform API versioning, contracts, operational references and route tests.
- Account creation, domain validation, ownership, verified-email gate and safe errors.
- Additive idempotency migration, atomic insert/replay/conflict and isolated tests.

## Out of scope

Email delivery/verification challenges, recovery, opening balances (P05.03),
listing, balances, renaming and archive operations. No financial gate bypass.
The register → verify → create journey remains unavailable until verification runtime ships.

## Constraints and invariants

Reuse domain validation, bearer identity, typed errors and isolated PostgreSQL.
Never accept owner/balance from clients. Retain original creation result for the
account lifetime. Rejected requests must leave account/idempotency data unchanged.

## Steps

- [x] Uniform `/v1` migration and regression tests.
- [x] Atomic account creation and lifetime idempotency with HTTP tests.
- [x] Own checks, independent high-risk verification and native review (approved and acknowledged).

## Acceptance criteria

- [x] Every current endpoint is versioned with unchanged non-path behavior.
- [x] Verified owners can create normal/savings accounts in supported currencies.
- [x] Invalid/unauthenticated/unverified inputs cannot create records.
- [x] Replays return the original result, conflicts write nothing, concurrent retries create once.
- [x] Contracts and development instructions describe implementation and remaining limitations.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 scripts/check_repository.py` | Repository consistent | Passed (230 files) | Pass |
| Pinned `cargo fmt`, `cargo clippy`, `cargo test` | Clean, all ordinary tests pass | fmt/Clippy passed; 75 Cargo tests passed, 46 DB tests ignored | Pass |
| Disposable PostgreSQL infrastructure harness | Existing and new integration tests pass | 47/48 full harness; fixture-only correction then seven account cases passed; full suite not rerun | Qualified pass |
| Independent verification | No change-caused blockers | PASS S1–S4; eight baseline HTTP differential cases matched | Pass |

## Risks, assumptions, and decisions

User explicitly approved breaking all route paths now, mandatory idempotency,
and lifetime retention. Risk: items 2/3/4/5 (security, API contract, concurrency,
and additive migration/dependency wiring). No production or remote access.
Single-PR size exception approved explicitly for approximately 1,500 authored
lines. No publication authorization.

## Completion notes

Implemented in `13cf1cbc4dbcd9a845a94123d448c83b30870e4f`; own/independent
checks passed. Native four-lens review approved with no blockers and exact
acknowledgement burned its authority (`review-cebc9d6c7dec4797`). Single-PR
size exception approved. No push or PR authorized or performed.
One non-blocking README prefix inconsistency was fixed in a separate passive
documentation unit. The review was not reopened.
The first full database run exposed a fixture-order bug in one new test; its
focused correction and the retained independent probe passed (seven cases).
Do not mark Phase 05 complete or claim verification is usable based on seeded
identities. See task log for counts, the one-pass slicing result and limitations. Keep this
record under `active/` until publication and merge are confirmed.
