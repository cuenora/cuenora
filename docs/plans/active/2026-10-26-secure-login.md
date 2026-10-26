# Execution plan: Secure login and initial opaque sessions

- **Status:** Active
- **Date:** 2026-10-26
- **Roadmap task ID(s):** P04.03; initial issuance prerequisites for later P04.04/P04.05
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Verify an existing password and issue a server-backed initial session with opaque
access (15 minutes) and refresh (30 days) credentials. Store digests only.
Pending users may authenticate without financial authority.

## Existing state and evidence

P04.02 implements registration, Argon2id hashing, strict email policy and bounded
blocking work. Migration 0002 has session families and refresh digests but no
access-token storage. Login, rotation, middleware and email verification are absent.

## In scope

- Exact password verification, safe authentication failures and shared CPU admission.
- Opaque credential generation, forward access persistence migration and atomic issuance.
- Public HTTP/disposable PostgreSQL tests and synchronized OpenAPI/docs.

## Out of scope

Refresh rotation/revocation endpoints, identity middleware, financial access,
verification/recovery email, deployment, production rate limiting and publication.

## Constraints and invariants

Follow AGENTS.md and the approved S1–S5 in `odd/tasks/p04-03-login.md`. Never
persist/log raw session credentials or plaintext passwords. Preserve registration,
liveness, existing identity data and financial-domain independence. No automatic
P04.04/P04.05/P04.12 completion.

## Steps

- [ ] T1 credential verification and opaque token primitives with RED/GREEN proof.
- [ ] T2 access storage and atomic initial issuance with rollback proof.
- [ ] T3 public login and unchanged registration/liveness behavior.
- [ ] T4 independent security/concurrency/migration verification and native reviews.

## Acceptance criteria

- [ ] All approved S1–S5 have observed public-interface proof.
- [ ] Failed authentication/issuance leaves stored state unchanged.
- [ ] Pending identity is authenticated without verification or financial authority.
- [ ] No raw credentials are stored or exposed through diagnostics.
- [ ] OpenAPI, setup and roadmap reflect actual verified behavior.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Cargo fmt/Clippy/workspace tests | Pass without warnings | T1: 36 runtime + six compile-fail tests, fmt and Clippy passed | Passed for T1 |
| Focused public login and disposable PostgreSQL tests | Real session, generic failures, atomic rollback | Not implemented | Pending |
| Existing registration/liveness differential | Unchanged responses and persisted effects | Not run | Pending |
| Repository/OpenAPI checks | Synchronized valid artifacts | T1 repository validation passed (177 files); no HTTP contract change yet | Partial |
| Independent verification and locked dependency audit | No reproducible blockers | T1 independent probes passed; no package/version/checksum changes. Full feature audit pending | Partial |

## Risks, assumptions, and decisions

Risk items 1 (forward migration), 2 (authentication/credentials), 3 (shared route
helpers), 4 (cancellation/issuance ordering), and 5 if dependency changes apply.
User approved opaque credentials and the durations, with a new persistence
migration. ODD tracking and local slices are authorized; the maintainer approved stacked-to-main chained slices. Later remote publication
and size exceptions remain separate decisions.

## Completion notes

Started from main `a207911`. T1 pure verification/token primitives are implemented and own checks passed.
No P04.03 migration, session persistence or login endpoint exists yet.
The ODD feature document is the full specification and resumption logbook.

T1 independent probes and parent spot check passed. Ten baseline/current public
router scenarios matched exactly; six privacy compilation checks are retained.
Stacked-to-main chain strategy approved. First local work-unit commit and
its candidate-specific native review are next; no publication authorized.
