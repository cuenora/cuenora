# Execution plan: Align roadmap with product decisions

- **Status:** Completed
- **Date:** 2026-10-22
- **Roadmap task ID(s):** P00.10 follow-through; existing future task IDs retained
- **Tracking issue / PR:** [#20](https://github.com/cuenora/cuenora/pull/20) — merged 2026-10-22
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Align the master roadmap and product backlog with the completed product-definition
documents before preparing a PR. Preserve verified work and avoid new scope.

## Existing state and evidence

A read-only roadmap review found stale sprint/next-document guidance, an implicit
funding-gap implementation task, a missing optional transfer-rate field in the
proposed model, and a post-MVP rate-change task duplicating accepted MVP scope.
The existing Money test remains future work; the completed P02.09 test is a
scaffold unit test, not evidence of a financial Money implementation.

## In scope

- Roadmap wording, acceptance cases, dependency order, and matching backlog links.
- Preserve all existing task IDs and checked implementation evidence.

## Out of scope

- Source implementation, new product features, commit, push, and PR creation.

## Constraints and invariants

SEK/EUR/USD; actual cross-currency transfer amounts with optional rate; savings
opening/transfer/credited-interest funding; daily payouts and effective-dated
rates; in-app funding comparisons; reminders remain later work.

## Steps

- [x] Review the roadmap against accepted product documents.
- [x] Reconcile tasks, scope, proposed contracts, and current next steps.
- [x] Verify task identities, completion preservation, links, and consistency.

## Acceptance criteria

- [x] Existing IDs remain traceable; superseded tasks are explained, not erased.
- [x] Pending implementation is distinct from completed product documentation.
- [x] Backlog and roadmap explicitly assign the account-funding comparison.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 scripts/check_repository.py` | Pass | Passed | Passed |
| `git diff --check` | Pass | Passed | Passed |
| IDs, checkboxes, relative links, product consistency | Pass | Passed | Passed |

## Risks, assumptions, and decisions

Calculation precision, rate interpretation, calendar policies, and planning
metric boundaries remain specification prerequisites. No unsupported bank
conditions are inferred. Phase numbering is retained; dependency order is clarified.

## Completion notes

Updated master plan v0.3, backlog mappings, and planning index. Preserved all
186 original IDs in order and all 25 checked statuses from the pre-alignment
working tree. P15.06 remains an explicitly superseded unchecked alias; it does
not block MVP. Added FIN-017–FIN-022 without changing the prior 16 cases.
Repository validation, whitespace, IDs/status preservation, relative-file links,
and structural consistency checks passed. No financial code changed or application
tests run.

## Delivery decision

The maintainer explicitly authorized one documentation PR exceeding 400 changed
lines on 2026-10-22. Delivery strategy: `exception-ok`; review starts with
`docs/product/prd.md`, then the backlog and master-plan changes. The complete
product definition and roadmap alignment form one documentation work unit.
Runtime validation is not applicable: no application source or dependency changes.
Rollback boundary: this product-documentation and roadmap-alignment commit.

## Merge and archive evidence

- Product documentation was committed as `ee85f1f` and merged through
  [PR #20](https://github.com/cuenora/cuenora/pull/20) on 2026-10-22 at 14:56:35 UTC.
- Merge commit: `312d4789a30b9ad1f2607874bff6c027186533eb`.
- Hosted checks all passed: Repository / Validate, Backend / Rust,
  Backend / Audit, and Backend / Dependency Review.
- The local feature and merged-main trees were verified identical before archival.
- Archived under `docs/plans/completed/` after merge, following the planning convention.
  This closes the documentation work; financial implementation remains pending.
