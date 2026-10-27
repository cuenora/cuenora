# Execution plan: Authenticated identity middleware

- **Status:** Active; implementation verified locally, publication and merge pending
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
- [x] T2: Run own checks, independent security verification and applicable native review.

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
| Native candidate-specific review | Inspect immutable security unit | Four concurrent perspectives returned no findings; exact acknowledgement confirmed authority burned | Passed |

## Risks, assumptions, and decisions

Risk: item 2 (authentication boundary). Test-only protected handlers avoid an
unrequested public endpoint. A successful lookup admits the request at its
database snapshot; later revocation does not cancel in-flight handlers.

## Completion notes

Implementation is committed at `75132b6` (821 authored lines, 12 files). The
maintainer approved a single-PR size exception, without publication. Own and
independent functional checks passed. The full financial infrastructure suite
was not run; four relevant DB harnesses were selected instead.

The maintainer separately granted the HIGH native review. All four perspectives
returned no findings; no correction was needed. Exact acknowledgement confirmed
authority burned for the immutable implementation candidate. Review inspected
patches, not a replacement for executed functional tests.

Only P04.05 is marked complete locally. P04.06–P04.12 and the Phase 04 exit remain
open. This final closure changes passive documentation only; reviewed runtime,
tests, schema, dependencies and contracts remain unchanged. No push or PR yet;
publication needs separate authorization. Archive only after merge.
