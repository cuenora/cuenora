# Execution plan: Sliding refresh rotation and session revocation

- **Status:** Active; stacked-to-main strategy approved, publication not authorized
- **Date:** 2026-10-27
- **Roadmap task ID(s):** P04.04
- **Tracking issue / PR:** Not created; publication not authorized
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Refresh a session using one-time opaque refresh credentials. Every successful
rotation renews refresh validity for 30 days, without an absolute session-age cap.
Reusing a consumed credential revokes its session family, not other sessions.

## Existing state and evidence

P04.03 provides public login, independent opaque access/refresh secrets, digest-only
atomic issuance and 15-minute/30-day lifetimes. Migration 0002 retains refresh
generations and session revocation; 0009 stores access digests. Neither refresh
rotation nor bearer middleware is implemented at this plan's starting boundary.
See the full specification and recovery log in
[`odd/tasks/p04-04-session-rotation.md`](../../../odd/tasks/p04-04-session-rotation.md).

## In scope

- Transactional sliding refresh rotation, replay-triggered family revocation and
  an internally ownership-scoped session revocation boundary.
- Public refresh HTTP behavior, digest-only persistence and synchronized contract.
- Focused isolated PostgreSQL/HTTP checks, failure atomicity, expiry and races.

## Out of scope

Passkeys, mobile implementation, bearer identity middleware, financial ownership
routes, logout/global revocation endpoints, email, deployment and publication.

## Constraints and invariants

Follow root AGENTS.md and S1–S5 in the ODD specification. Preserve initial login,
registration, liveness and existing records. Serialize operations per family;
never trust caller owner IDs or persist/log raw secrets. Rotation alone does not
grant financial authority. Retain history and existing dependency/schema formats.

## Steps

- [ ] T1: Implement the refresh/session adapter with tests and its decision record.
- [ ] T2: Expose refresh HTTP with tests and synchronized OpenAPI/documentation.
- [ ] T3: Independently verify security/concurrency and close observed acceptance.

## Acceptance criteria

- [ ] Each valid refresh is consumed once and replaced atomically with new secrets.
- [ ] Successor refresh expires 30 days after rotation, even for an older family.
- [ ] Expired/revoked/invalid credentials cannot issue a new pair.
- [ ] Reuse revokes only the owning session, with deterministic race handling.
- [ ] Failed writes roll back; secrets never enter storage or diagnostics.
- [ ] Existing login/registration/liveness behavior remains unchanged.
- [ ] Public contract and setup match verified behavior; later tasks stay pending.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Cargo fmt, Clippy and workspace tests | Pass without warnings | Final T1: 47 including doctests; 23 DB cases intentionally ignored | Passed for T1 |
| New disposable PostgreSQL store/HTTP targets | Rotation, replay, expiry, rollback and races pass | Final T1: six adapter groups passed, 16.999 s; HTTP pending | Partial |
| Full disposable PostgreSQL infrastructure suite | Existing and new storage cases pass | T1: 43 passed, 854.491 s; before supplemental test-only addition | Passed for T1 production source |
| Baseline/current registration, login and liveness differential | Unchanged responses and effects | Nine old-route scenarios matched exactly; parent rerun passed; no preexisting runtime source changed | Passed for T1 |
| `python3 scripts/check_repository.py` | Valid repository and documentation | Passed, 186 Git-visible files at planning boundary | Passed for planning only |
| Independent verifier and parent spot check | S1–S5 covered, no change-caused blockers | T1 adapter accepted with no blockers; T2 explicitly pending | Passed for T1 |

## Risks, assumptions, and decisions

Risk items 1 (persisted session mutation), 2 (credentials), 3 (shared router) and
4 (transaction/concurrency ordering). Sliding expiry is explicitly approved for
convenience until future passkey work; it does not eliminate token theft risk.
Consumed-token reuse intentionally forces fresh login for the whole family,
including legitimate concurrent refresh/retry. App activity must actually refresh
to extend validity. No new schema or dependency is currently expected.

## Completion notes

T1 adapter and its tests/docs are implemented with own and independent checks
passed; native review remains pending. Public HTTP is not
implemented yet. Stacked-to-main delivery is approved; publication and oversized
PR exceptions are not. Keep coherent tests/docs with each behavior slice.
Keep this execution record active until verified implementation is merged.
