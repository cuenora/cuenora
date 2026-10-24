# Execution plan: Account storage and ownership

- **Status:** Completed and merged
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.05
- **Tracking issue / PR:** [PR #29](https://github.com/cuenora/cuenora/pull/29)
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Add reproducible account storage for the ownership roots of later financial
records. This slice implements database storage; account actions remain Phase 05.

## Existing state and evidence

P03.04 is merged as `4545720`. Users, credentials and sessions have versioned SQL
and an isolated PostgreSQL harness. Accounts had no table at this baseline.
Normal/Savings, SEK/EUR/USD and movement-derived balances are already agreed.

## In scope

One account migration, isolated acceptance tests and storage documentation.
The full specification and proof log are in
[the ODD record](../../../odd/tasks/p03-05-accounts-storage.md).

## Out of scope

SQLx/API access, row authorization, opening postings, balances, account action
protocols, name uniqueness and post-history type/currency mutation policies.

## Constraints and invariants

Reuse existing user identifiers, UTC audit storage and disposable Compose helpers.
No financial data, client balance or speculative child table. Implementation used
a separate checkout to preserve the then-pending P03.04 archive.

## Steps

- [x] 1. Add account acceptance tests and observe RED before SQL.
- [x] 2. Implement migration/docs and observe focused GREEN.
- [x] 3. Run the full suite, independent verification and native assessment.

## Acceptance criteria

- [x] Valid account rows persist supported types/currencies and archive state.
- [x] Invalid storage/ownership writes fail without changing persisted data.
- [x] Future owner-scoped references reject cross-user relationships.
- [x] Failed migration DDL rolls back, and existing behavior remains intact.

## Verification

| Command/scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused account tests before SQL | Missing-schema failures | Four failures, 20.920 s, before SQL | Passed |
| Focused account tests after SQL | Pass | Four account tests passed, 47.111 s | Passed |
| Full isolated PostgreSQL suite | Existing and new tests pass, no leaked resources | Eleven passed, 116.625 s; cleanup passed | Passed |
| Independent/baseline probes | No change-caused blocker | S1–S6 verification passed; two independent probes passed | Passed |
| Repository/link/whitespace checks | Pass | All passed | Passed |
| Granted native candidate review | Close with exact acknowledgement | Four lenses, zero findings; acknowledgement confirmed authority burned | Passed |
| Hosted PR checks | All four checks pass | Repository / Validate, Backend / Rust (including PostgreSQL), Backend / Audit and Dependency Review succeeded | Passed |

## Risks, assumptions, and decisions

Risk: items 1 (migration), 2 (ownership integrity). Foreign keys do not authenticate
requests. Future application code must enforce current-user ownership and gates.

## Completion notes

Implementation commit `1a7ef54` passed focused tests, the full eleven-test
PostgreSQL suite and two independent ownership/rollback probes. All four native
reviewers returned zero findings; exact acknowledgement closed the review.
Existing Python methods and backend files match baseline; Rust was not rebuilt locally
because its source and dependencies were unchanged; hosted workspace checks passed.

PR #29 merged on 2026-10-24 at 15:46:29 UTC as
`95fa1f3a9c3f72af6fc680e39fea62d8344eff1c`, with all four PR checks successful.
The merged tree matches the verified published head `735292b`. P03.05 is complete
and this plan is archived after merge. Account application actions, posted
balances and later financial tables remain separate roadmap units. P03.06 is next
and has not started.
