# Execution plan: Minimal user and session storage

- **Status:** Complete locally; awaiting PR and merge
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.04
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Provide reproducible PostgreSQL migrations for minimal user identity, credentials
and session state with real isolated database tests. This does not provide login,
registration, mail delivery or financial endpoints.

## Existing state and evidence

Money is implemented; dates and first-party/open/verified identity direction are
defined. No application schema or SQLx dependency existed at baseline. The existing Compose
smoke-test harness passed both baseline tests before edits. The user approved
case-insensitive email identity, original-address preservation and ASCII local
parts with IDNA domains while Resend SMTPUTF8 support remains unconfirmed.

## In scope

Versioned SQL, isolated PostgreSQL tests, a CI entry point and storage documentation.
The full recovery specification and test log are in
[`odd/tasks/p03-04-user-identity-storage.md`](../../../odd/tasks/p03-04-user-identity-storage.md).

## Out of scope

SQLx installation, API database connections, token issuance, hashing passwords,
Resend integration, email challenges, financial tables and full auth protocols.

## Constraints and invariants

Use local disposable databases and synthetic data only. Reuse the existing Compose
helpers and their cleanup/password handling. Preserve `.atl/`, Money and liveness.
No raw token storage or silent canonicalization guesses at the API boundary.

## Steps

- [x] 1. Add migration acceptance tests and observe RED before writing SQL.
- [x] 2. Implement minimal migrations, documentation and CI; observe GREEN.
- [x] 3. Obtain independent verification and applicable native assessment/review.

## Acceptance criteria

- [x] Fresh migrations create the documented identity/session structures.
- [x] Key/ownership/lifecycle constraints reject invalid writes without state changes.
- [x] Migration failure rolls back partial DDL, and existing tests still pass.
- [x] Documentation states which application protections remain pending.

## Verification

| Command/scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Existing Compose smoke tests, before edits | Existing two tests pass | Passed, 20.269 s | Passed |
| Focused migration tests before SQL | Fail for missing schema | Five failures, 32.731 s | Passed |
| Full Compose/integration suite after SQL | All existing/new tests pass with cleanup | Seven passed, 68.965 s | Passed |
| Repository/link/whitespace checks | Pass | Repository, local links and whitespace passed | Passed |
| Independent verification | No change-caused blocker | S1–S7 passed; independent rollback/rejection probes passed | Passed |
| Native candidate review | Granted review closes with acknowledgement | Four lenses, zero findings; acknowledgement confirmed authority burned | Passed |

## Risks, assumptions, and decisions

Risk: items 1 (migrations), 2 (identity/session integrity), 5 (CI integration).
Storage constraints are not a substitute for server authentication, authorization,
IDNA/email validation, secure token generation or session rotation protocols.

## Completion notes

Implementation commit `7d526ae` passed functional verification, independent S1–S7
verification and the granted four-lens native review with zero findings. Exact
acknowledgement closed the native transaction. Rust tests (21), format and Clippy
passed; the complete PostgreSQL suite passed all seven tests with resource cleanup.
The maintainer accepted size:exception for one cohesive delivery unit.

P03.04 is complete locally. Hosted CI, publication and merge remain pending. No
push or PR has been performed. Keep this plan in `active/` until its future PR is
merged. Financial tables and Phase 04 authentication remain separate work.
