# Execution plan: Token rejection verification

- **Status:** Completed
- **Date:** 2026-10-28
- **Roadmap task ID(s):** P04.09
- **Tracking issue / PR:** [PR #63](https://github.com/cuenora/cuenora/pull/63)
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Verify P04.09: “Test expired/revoked tokens and refresh-token reuse.”
Existing acceptance tests cover this requirement; this task records a fresh
execution against disposable PostgreSQL without changing authentication behavior.

## Existing state and evidence

Verified baseline: `8defc21` on local `main`.

| Boundary | Existing acceptance coverage |
| --- | --- |
| Session adapter, `backend/crates/api/tests/session_store.rs` | Expired and unknown refresh rejection without writes; explicit revocation and replay scoped to a family; concurrent rotation/revocation, cancellation and rollback. |
| Public refresh HTTP, `backend/crates/api/tests/refresh_http.rs` | Generic 401 for expired/replayed/revoked tokens; rejected expiry does not change storage; replay revokes its family and rejects the successor, while sibling and other-user sessions remain usable. |
| Protected HTTP middleware, `backend/crates/api/tests/auth_identity.rs` | Expired access rejection without writes; replay invalidates both still-unexpired access generations; another session remains usable; explicit revocation invalidates its access. |

The protected route is a test-only fixture, not a production resource endpoint.
These database-backed tests are ignored by ordinary Cargo runs; the Compose
harness explicitly executes them and checks their expected passing counts.

## In scope

- Run the existing session adapter, refresh HTTP and identity middleware groups.
- Record observed results and update the roadmap's P04.09 completion evidence.

## Out of scope

New endpoints, changed token semantics, new dependencies, financial ownership
tests (P04.06/P04.10), logout/global revocation endpoints (P04.11), and email
verification/password recovery (P04.12).

## Constraints and invariants

Use only disposable local PostgreSQL and synthetic identities. Preserve generic
authentication errors, unchanged storage on non-replay rejection, and family-only
replay revocation. Follow ADR-016, ADR-017 and ADR-018; no new decision is needed.

## Steps

- [x] 1. Map existing coverage to P04.09 and inspect representative assertions.
- [x] 2. Execute the three disposable PostgreSQL harness targets.
- [x] 3. Record local verification and update the roadmap.

## Acceptance criteria

- [x] Expired access and refresh credentials are rejected.
- [x] Revoked credentials cannot authenticate or refresh.
- [x] Consumed refresh-token reuse revokes its own family, including successor
  refresh and still-live access credentials, without revoking unrelated sessions.
- [x] Existing tests run against isolated PostgreSQL rather than remaining ignored.

## Verification

Run from the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_session_store_against_disposable_postgres \
  PostgresComposeTest.test_refresh_http_against_disposable_postgres \
  PostgresComposeTest.test_auth_identity_against_disposable_postgres -v
```

Observed on 2026-10-28: **PASS**, all three harness targets in **82.346 seconds**.
The harness required **6 session adapter + 5 refresh HTTP + 3 identity middleware
tests = 14 passing integration tests**. Cleanup checks passed for each temporary
project's containers, volumes and networks.

## Risks, assumptions, and decisions

- No runtime, test, migration, dependency or API contract changes were necessary.
- RED/GREEN is not applicable: no behavior or test was added or modified.
- Verification is scoped to P04.09; it does not certify Phase 04 completion or
  production readiness. The full repository/infrastructure suite was not rerun.

## Completion notes

Only planning documentation changed. [PR #63](https://github.com/cuenora/cuenora/pull/63)
merged into `main` on 2026-10-28 as `bcc9e8e`. The full merged tree was verified
identical to published commit `149a699`; local `main` was fast-forwarded to the merge.
This record is archived after verified merge, preserving the original test evidence.
The archival documentation update is separate from the original verification PR.
