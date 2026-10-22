# Execution plan: PR dependency review

- **Status:** Active
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.11
- **Tracking issue / PR:** Not created
- **Owner:** Codex

## Goal and user-visible outcome

Review dependency changes in PRs with read-only permissions, no automatic comments,
and no additional license policy, complementing the full RustSec audit.

## Existing state and evidence

Main requires PRs and all three existing CI checks. The maintainer enabled the
dependency graph after comparison API requests returned 403. A subsequent live
`main...main` comparison returned HTTP 200 and an empty array.

## In scope

- Add a PR-only Dependency Review workflow and document its policy.
- Verify configuration and comparison API availability.

## Out of scope

- Runtime dependencies, financial features, license restrictions, automatic comments.
- Publishing a PR or changing required checks without delivery authorization.

## Constraints and invariants

Preserve existing CI checks and user files. Use contents: read only. Fail real
vulnerability findings and API errors; document non-blocking snapshot warnings. Do not claim fixture tests prove the
hosted service accepts or rejects an actual dependency-changing PR.

## Steps

- [x] 1. Verify existing protection and graph API availability.
- [x] 2. Add workflow and update dependency policy.
- [x] 3. Run local checks and independent verification.
- [ ] 4. Observe hosted safe/vulnerable behavior and finalize required-check policy.

## Acceptance criteria

- [x] PR-only check uses read-only permissions, no comments and no license checks.
- [x] Existing jobs and permissions remain unchanged.
- [ ] Hosted safe and vulnerable scenarios have observed outcomes.
- [ ] Required-check decision is recorded; P02.11 remains pending until verified.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Graph compare main...main | HTTP 200, empty array | Passed | Pass |
| Repository validator / whitespace | Pass | Passed | Pass |
| Independent workflow review | No blockers | Both scope/documentation findings corrected and rechecked | Pass |
| Hosted safe/vulnerable PRs | Pass / fail | Not run | Pending |

## Risks, assumptions, and decisions

- Risk: items 2 and 5 (security check and CI action).
- Action v5, explicit low severity and all scopes, comments never, license-check false.
- Snapshot warnings are non-blocking upstream; document incomplete coverage honestly.
- No meaningful local RED test exists for GitHub's hosted comparison service.
  Local structural checks are partial proof; real PR execution remains required.

## Completion notes

Implementation is local and unpublished. P02.11 is not marked complete.
