# Execution plan: Secure login and initial opaque sessions

- **Status:** Published as four chained PRs; CI and maintainer merge pending
- **Date:** 2026-10-26
- **Roadmap task ID(s):** P04.03; initial issuance prerequisites for later P04.04/P04.05
- **Tracking issue / PR:** No issue (explicitly authorized); [#48](https://github.com/cuenora/cuenora/pull/48) → [#49](https://github.com/cuenora/cuenora/pull/49) → [#50](https://github.com/cuenora/cuenora/pull/50) → [#51](https://github.com/cuenora/cuenora/pull/51)
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Verify an existing password and issue a server-backed initial session with opaque
access (15 minutes) and refresh (30 days) credentials. Store digests only.
Pending users may authenticate without financial authority.

## Starting state and evidence

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

Follow AGENTS.md and the approved S1–S6 in `odd/tasks/p04-03-login.md`. Never
persist/log raw session credentials or plaintext passwords. Preserve registration,
liveness, existing identity data and financial-domain independence. No automatic
P04.04/P04.05/P04.12 completion.

## Steps

- [x] T1 credential primitives: own/independent checks and native review acknowledged.
- [x] T2 access storage and atomic initial issuance with rollback proof.
- [x] T3 public login and unchanged registration/liveness behavior; native review acknowledged.
- [x] T4 independent verification, dependency audit and native reviews passed.

## Acceptance criteria

- [x] All approved S1–S6 have observed public-interface proof.
- [x] Failed authentication/issuance leaves stored state unchanged.
- [x] Pending identity is authenticated without verification or financial authority.
- [x] No raw credentials are stored or exposed through diagnostics.
- [x] OpenAPI, setup and roadmap reflect actual verified behavior.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Cargo fmt/Clippy/workspace tests | Pass without warnings | T3: 37 runtime + ten compile-fail tests, fmt and Clippy passed | Passed for current slice |
| Focused public login and disposable PostgreSQL tests | Real session, generic failures, atomic rollback | Final five HTTP cases passed (19.599 s); full infrastructure 42 passed (850.281 s) before supplemental test-only additions | Partial |
| Existing registration/liveness differential | Unchanged responses and persisted effects | Independent 13 baseline/current scenarios and normalized registration effects matched; parent spot check passed | Passed |
| Repository/OpenAPI checks | Synchronized valid artifacts | T3 repository validation passed (184 files); OpenAPI and docs synchronized | Passed |
| Independent verification and locked dependency audit | No reproducible blockers | Independent security/concurrency proof passed; cargo-audit 0.22.2 scanned 191 dependencies without warnings/advisories | Passed |

## Risks, assumptions, and decisions

Risk items 1 (forward migration), 2 (authentication/credentials), 3 (shared route
helpers), 4 (cancellation/issuance ordering), and 5 if dependency changes apply.
User approved opaque credentials and the durations, with a new persistence
migration. ODD tracking and local slices are authorized; the maintainer approved stacked-to-main chained slices. Later remote publication
and size exceptions remain separate decisions.

## Completion notes

Started from main `a207911`. T1 pure verification/token primitives are implemented and own checks passed.
Access storage, initial issuance and HTTP login are implemented; all native candidate reviews were acknowledged without findings.
The ODD feature document is the full specification and resumption logbook.

T1 independent probes and parent spot check passed. Ten baseline/current public
router scenarios matched exactly; six privacy compilation checks are retained.
Stacked-to-main chain strategy approved. First local work-unit commit is `d926f57`;
its candidate-specific native review completed without findings and was acknowledged; no publication authorized.

T2A access storage is implemented: full PostgreSQL infrastructure suite 40 passed
(798.924 s), independent migration/ownership/transactional probes passed, and the
final strengthened two focused cases passed afterward (36.653 s). Existing data
and constraints were preserved. The migration alone grants no authority. At that storage-only slice HTTP login remained pending. Native T2A review completed without findings and was acknowledged.

T2A local commit: `8c046fff7746caf8cf882ca1eec598bc538fad46` (222 changed lines).
The exact native review was acknowledged without findings; no publication authorized.

T2B credential lookup and atomic initial session/access/refresh issuance are
implemented. Parent reproduced and corrected locale-dependent lookup using a
Turkish ICU database and explicit C input casing. Final focused adapter cases
passed (15.819 s); full infrastructure passed 41 cases (830.281 s), followed
by supplemental test-only regressions. Independent security/concurrency proof
and parent spot check passed; fmt/Clippy and 36 runtime plus ten compile-fail
tests passed. Native candidate review completed without findings and was acknowledged. This is a Rust persistence boundary, not a public login
endpoint or bearer authorization. No publication authorized.

T2B local commit: `eadd6f814582b45ec163969e7e379a683f260151` (610 authored changed lines).
Its separate native review completed without findings and was acknowledged.
No push, PR, merge or size exception is authorized.

T3 public HTTP login and shared admission are implemented and independently
verified. The approved response contract, safe failures, pending-user isolation,
real stale-credential race and mixed-route saturation are covered. Thirteen
baseline/current registration/liveness scenarios and normalized persisted effects
matched. Final five HTTP cases passed (19.599 s), alongside pinned fmt/Clippy,
37 runtime and ten compile-fail tests. The full 42-case infrastructure run
(850.281 s) preceded supplemental test/doc-only regressions. Audit scanned 191
dependencies with no warnings/advisories. One ADR contradiction was corrected
and passed bounded independent readback. Native T3 review was acknowledged without findings;
P04.03 is checked complete for the verified local implementation. No publication authorized.

T3 local commit: `d0fb2ec396a3e3210f64b2ceb7c7289c072e7a9a` (940 authored changed lines).
Its high-risk native review was explicitly granted, completed without findings
and acknowledged. Publication and size exceptions remain separate decisions.
This plan stays in `active/` until the corresponding PRs are merged, per the
planning policy. P04.04–P04.12 and the Phase 04 exit remain incomplete.

The maintainer explicitly authorized the four-PR stacked publication, no linked
issue, and exceptions for slices exceeding 400 lines. This permission is scoped
to P04.03; merge remains a separate decision. Earlier no-publication wording
above records the implementation-time boundary.

Publication confirmed: #48 targets main, #49 targets the credential branch, #50
targets access storage, and #51 targets issuance. All are open with linked chain
context and explicit issue/size exceptions. CI is pending on the published heads;
no merge is authorized or performed. Retarget the next PR only after its parent
lands, then verify its diff and checks again.
