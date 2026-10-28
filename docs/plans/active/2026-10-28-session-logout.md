# Execution plan: Logout and global session revocation

- **Status:** Implemented and independently verified locally; commit/native review pending
- **Date:** 2026-10-28
- **Roadmap task ID(s):** P04.11
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Implement session logout using retained refresh possession and global logout
using valid access plus the current password, as approved by the maintainer.
See [ADR-021](../../architecture/decisions/021-session-logout.md) for the contract
and `odd/tasks/p04-11-session-revocation.md` for the implementation log.

## Existing state and evidence

Baseline `7aca6d9` includes family-scoped revocation, refresh rotation and bearer
identity middleware. Login locks the credential row before initial issuance.
There are no logout routes or global revocation adapter at baseline.

## In scope

Two logout endpoints, transactional global revocation, abuse/admission protection,
OpenAPI/setup documentation and isolated HTTP/concurrency regression tests.

## Out of scope

New schema/dependencies, password recovery, email verification, financial APIs,
resource ownership (P04.06/P04.10), remote publishing and production deployment.

## Constraints and invariants

Only server-derived ownership; exact password verification; generic errors;
retain token history; atomic failure; serial cutoff with login/refresh;
unchanged existing registration/login/refresh contract.

## Steps

- [x] Map existing helpers and confirm the logout authorization/cutoff policy.
- [x] Observe failing logout transport and disposable database tests.
- [x] Implement and normalize the approved behavior with regression tests/docs.
- [x] Run own checks and independent security/concurrency verification.
- [ ] Record delivery evidence; native review only when due and consented.

## Acceptance criteria

- [x] Idempotent logout revokes only the refresh credential's family.
- [x] Valid access plus the current password revokes all that user's sessions.
- [x] Other users and subsequent logins remain unaffected.
- [x] Concurrent login/refresh and transaction failures preserve the contract.
- [x] Existing endpoints remain compatible; typed errors, limits and no-store hold.

## Verification

Focused command:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_logout_http_against_disposable_postgres -v
```

Full checks: repository validator; pinned Cargo fmt/clippy/workspace tests;
full disposable PostgreSQL suite; independent isolated probes. Observed on 2026-10-28:

| Check | Result |
| --- | --- |
| Pinned fmt, Clippy all targets/features and workspace tests | PASS; 68 tests passed, 39 DB tests intentionally ignored by direct Cargo after final regression adoption. |
| Full disposable PostgreSQL suite | PASS; 47 harness tests in 941.182s, including the original five logout DB groups; disposable resources cleaned. |
| Independent public-interface verification | PASS; seven logout DB groups in 40.985s, including two additional regression groups; no blockers. |
| Parent final regression spot check | PASS; all seven logout DB groups in 40.584s after adoption; full PostgreSQL suite not repeated for test-only additions. |
| Existing-route baseline/candidate differential | PASS; 14 normalized cases identical with separate Cargo targets. |
| OpenAPI YAML/local references, repository conventions, whitespace | PASS. |

The initial new transport and all four initial DB groups failed before runtime
implementation. The first concurrency test exposed observer-pool contention,
not a production bypass; a separate test observer pool resolved it. No runtime
correction was required by independent verification.

## Risks, assumptions, and decisions

Risk: items 2 and 4 (security and concurrent session lifecycle). No migration or
new dependency. P04.10 remains unchecked until real resource endpoints exist.

## Completion notes

Maintainer approved one oversized PR as a size exception, without publication.
No publication or phase completion claimed. Keep this record active until merge.
