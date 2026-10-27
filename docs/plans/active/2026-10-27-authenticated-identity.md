# Execution plan: Authenticated identity middleware

- **Status:** Active
- **Date:** 2026-10-27
- **Roadmap task ID(s):** P04.05
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Provide reusable Axum middleware that validates an opaque access credential and
attaches a trusted user/session identity to a protected handler.

## Existing state and evidence

Migration 0009 stores owner-linked access digests and expiry. P04.03 issues them;
P04.04 retains family revocation state. No protected production route exists.
Full specification and recovery log: [ODD record](../../../odd/tasks/p04-05-authenticated-identity.md).

## In scope

Route-scoped authentication, read-only token lookup, typed identity, negative
HTTP/database proof, integration documentation and an ADR.

## Out of scope

New production endpoints, ownership/financial authorization, logout/global
revocation, email, rate limiting, schema/dependencies and publication.

## Constraints and invariants

Use only existing access secrets and server-derived IDs. Pending users may
authenticate without financial authority. Preserve all existing public routes;
never persist/log presented credentials or renew sessions during authentication.

## Steps

- [x] T1: Observe RED, implement middleware and document its boundary.
- [ ] T2: Run own checks, independent security verification and applicable native review.

## Acceptance criteria

- [x] Valid pending/verified identities reach the protected handler with correct IDs.
- [x] Invalid/expired/revoked credentials never reach it; failures expose no secrets.
- [x] Authentication does not modify persisted data or existing public behavior.
- [x] Integration and bounded revocation-snapshot semantics are documented.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned fmt, Clippy and workspace tests | Pass | Final checks passed; 52 including 11 doctests, 31 intentionally ignored DB cases | Passed |
| Focused disposable PostgreSQL middleware target | Valid/negative cases pass | Three DB groups passed; parent rerun after retaining alternate-transport probes: 16.330 s | Passed |
| Repository validation | Pass | 196 Git-visible files passed before passive closure notes | Passed |
| Independent security verification and parent spot check | No blockers | S1–S3 passed; 17 DB cases across four focused harnesses, 24 baseline/current response matches; parent focused DB rerun passed | Passed |

## Risks, assumptions, and decisions

Risk: item 2 (authentication boundary). Test-only protected handlers avoid an
unrequested public endpoint. A successful lookup admits the request at its
database snapshot; later revocation does not cancel in-flight handlers.

## Completion notes

Implementation and own/independent functional verification passed. The full
financial infrastructure suite was not run; four relevant DB harnesses were
selected instead. Security changes still require native assessment/review when
applicable. The maintainer approved a single-PR size exception for this coherent behavior
unit (about 800 authored lines, mostly tests and documentation), without publication.
Local work-unit commit and native review remain pending; no push or PR yet. Keep the P04.05 master checkbox open until closure,
P04.06–P04.12 and phase exit open. Archive only after verification and merge.
