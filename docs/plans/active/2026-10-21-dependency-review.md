# Execution plan: PR dependency review

- **Status:** Active
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.11
- **Tracking issue / PR:** [PR #15](https://github.com/cuenora/cuenora/pull/15)
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

Published the initial implementation in PR #15 (`4e75611`). Dependency Review,
Repository / Validate, and Backend / Rust passed. Backend / Audit failed because
Tokio 1.54.0 was yanked; the check correctly enforced the existing policy.

Update only Tokio to non-yanked patch 1.54.1 in Cargo.lock. The manifest and
features remain unchanged. Pinned format, Clippy, locked tests (one API test),
and build passed after the update. Independent verification and a parent online
audit spot check passed with cargo-audit 0.22.2: 1,348 advisories, 42 dependencies,
exit 0, including live yank checks. The temporary tool/database directory was removed.
P02.11 is not marked complete; hosted vulnerable-case proof and required-check
configuration remain pending.
