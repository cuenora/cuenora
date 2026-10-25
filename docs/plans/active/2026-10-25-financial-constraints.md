# Execution plan: Financial storage constraints

- **Status:** Active
- **Date:** 2026-10-25
- **Roadmap task ID(s):** P03.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Complete currency/account compatibility, ordinary-income account-type guards and
transaction user/date indexing. Retain existing amount/kind/currency checks.

## Existing state and evidence

Migrations 0003–0007 already constrain row amounts, operation kinds, supported
currencies, owners and immutable postings. Cross-table currency and savings
restrictions were deferred. See [the task log](../../../odd/tasks/p03-10-financial-constraints.md).

## In scope

- Migration 0008, regression tests and storage documentation.
- Approved account changes only when all existing references remain valid.

## Out of scope

- Posting counts, event/movement amount/sign/membership equality, automatic
  auditing, APIs and completion of P03.11/P03.12.

## Constraints and invariants

Preserve old migrations and existing data. Validate every existing row; fail
atomically on incompatibility without automatic repair. Native composite FKs
protect concurrent writes. Movement snapshots need not match current events.

## Steps and acceptance criteria

- [x] Observe RED for each new rule, implement migration and observe GREEN.
- [x] Test ordinary-income create/edit/assignment, both currency endpoints,
  movement currency, permitted/rejected account edits and concurrent writes.
- [x] Verify valid populated migration, incompatible-data rollback and indexes.
- [x] Preserve prior tests; run independent checks and consented native review.

## Verification

| Check | Expected | Observed |
| --- | --- | --- |
| Focused `test_financial_*` groups | RED then GREEN | Four groups passed across initial run and corrected three-group recheck (105.734s); observed RED before migration |
| Full PostgreSQL suite and baseline | Prior behavior preserved | Candidate 33 passed (714.048s); baseline 29 passed (575.920s) |
| Repository validation and pinned Rust checks | Pass | Repository 135 files; Rust 1.99.1 fmt, Clippy and 21 tests passed |
| Independent S1/S2 verification | Pass | 81 checks, eight blocking interleavings, 12 atomic rejections, eight identical baseline SQL results |
| Native assessment/review | Follow returned transitions | High assessment; user granted; four lenses without findings; exact acknowledgement burned authority |

## Risks, assumptions, and decisions

Risk: items 1, 2 and 4 (existing data, owner relationships and concurrency).
The maintainer approved conditional type/currency changes; linked ordinary income
blocks savings reclassification even when voided. See [ADR-008](../../architecture/decisions/008-account-reference-compatibility.md).

## Completion notes

Implementation `8ced806` satisfies S1/S2 with all checks above passing. Native
lineage `review-a874c70796229a84` was acknowledged; no correction was needed.
No checks remain failed or skipped. No push or PR was requested. Keep this plan
active until the implementation PR is merged. P03.11/P03.12 and application
posting/auditing remain pending.
