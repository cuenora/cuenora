# Execution plan: bounded authentication throttling

- **Status:** Completed — PR #61 merged into `main` (`a62e02e`); execution record archived
- **Date:** 2026-10-27
- **Roadmap task ID(s):** P04.08
- **Tracking issue / PR:** [PR #61](https://github.com/cuenora/cuenora/pull/61); no linked issue, explicitly authorized by the maintainer
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Bound registration, login and refresh attempts before expensive work, with safe
429 responses and explicit trusted-client identity for Cloudflare → Traefik → API.
See the specification and log in [the feature document](../../../odd/tasks/p04-08-auth-throttling.md).

## Existing state at implementation start

The API had 8 KiB JSON limits, password/refresh concurrency admission and typed
errors, but no temporal rate limit or socket peer metadata. The binary listens
on loopback. No live deployment was inspected.

## In scope

- Bounded process-local fixed-window limits, safe errors and retry metadata.
- Explicit immediate-proxy trust and Cloudflare client IP validation.
- Regression, concurrency, expiry and isolated persistence checks; contract/setup updates.

## Out of scope

- Remote deployment, firewall changes, Redis, replicas, migrations, CAPTCHA and recovery.
- Production readiness, financial endpoints and replacing Cloudflare edge protections.

## Constraints and invariants

Preserve accounting/authentication state on throttle rejection, existing email
normalization, body/concurrency limits, generic credential errors and liveness.
No raw email, token, password or client-IP logging. Only synthetic fixtures.

## Steps

- [x] 1. Observe failing HTTP throttle tests before implementation.
- [x] 2. Implement one bounded security unit with synchronized counters.
- [x] 3. Update contract, setup and ADR; run own checks.
- [x] 4. Obtain independent security/concurrency verification and resolve blockers.
- [x] 5. Resolve delivery-size strategy, commit and follow native review if applicable.

## Acceptance criteria

- [x] S1–S3 in the feature document have observed verification.
- [x] Throttled requests cannot write identity/session state.
- [x] Below-limit behavior remains compatible; malformed proxy identity fails closed.
- [x] Deployment assumptions and restart/replica limitations are explicit.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Focused throttle tests | Budgets, trust, expiry, atomic admission | 5 HTTP tests and 3 limiter units passed; DB test runs in harness | Passed |
| Cargo fmt, Clippy and workspace tests | Pass on pinned compiler | Rust 1.99.1; fmt/Clippy clean; 66 passed, 32 deliberately ignored DB cases | Passed |
| Repository checker | Pass | 209 Git-visible files validated; OpenAPI YAML and all local references also checked | Passed |
| Disposable PostgreSQL suite | No authentication/storage regressions | All 46 cases passed (922.2 s); strengthened focused case passed afterward (15.479 s) | Passed |
| Independent verifier | S1–S3 verified | PASS; 34 TCP compatibility cases, real peer/parallel/expiry probes and focused DB proof; no blockers | Passed |
| Native immutable-patch review | No blocking findings | Four perspectives, no findings; exact acknowledgement confirmed authority burned | Approved |

## Risks, assumptions, and decisions

Risk: items 2 and 4 (security and synchronized admission). A single API instance
was confirmed by the maintainer. Process-local state resets at restart. Exact
trusted Traefik IPs must be configured; origin isolation remains operator-owned
and unverified. Numerical budgets are engineering defaults, not measured capacity.

## Completion notes

Own and independent functional verification passed. A registration persistence
probe was strengthened to use a fresh email (an accidental duplicate write could
otherwise be a no-op); the full five-table hash remained unchanged. Parent spot
checks passed the three limiter units and final focused PostgreSQL case.

The maintainer approved a single oversized PR without publication, then separately
selected “1” to grant native review of the frozen candidate. All four perspectives
(risk, resilience, readability, reliability) returned no findings. No correction
was needed. Exact acknowledgement confirmed authority burned for lineage
`review-b675f0cf3be0c83b`, consumed revision
`sha256:0a9738e31705b0fcffca179f6e47fbdd32ceb0524287e3d9bbd733608ad29b44`.
Reviewers performed static immutable-patch inspection; functional evidence is the
own and independent execution above, not newly executed reviewer tests.

The maintainer separately authorized publication to `cuenora/cuenora` using the
existing GitHub/SSH session and inclusion of these five passive records in a
second documentation-only commit. The maintainer subsequently merged
[PR #61](https://github.com/cuenora/cuenora/pull/61) into `main` as `a62e02e`.
The verified merge permits archival of this execution record. The full 46-case
suite preceded the final test-only fresh-email strengthening; production code
did not change. The authorized second docs-only commit carries the five passive
closure and publication records; the reviewed implementation is not rewritten.

Rollback boundary: revert the single authentication-protection unit with its
config, errors, HTTP regressions and documentation. Existing schemas, stored
identities, sessions and financial records are unchanged; no migration or
persistent counter cleanup is needed.

Local work-unit commit: `d1bb428970994957b6d168c60a1a8cc3d90ebbca` (21 files,
1,159 authored changed lines). Native HIGH review was separately granted and
acknowledged. Review approval is informational; publication was authorized separately.

The initial published head is `d1bb428`; the authorized second commit contains
only the five closure/publication records. Exact final head, labels and hosted
check state are preserved in the publication readback rather than recursively
adding another commit to record its own hash.

## Post-merge archival

The maintainer reported “mergeada”, then explicitly approved updating local
`main`, removing the merged implementation branch and archiving the plan on a
documentation branch. GitHub reported merge commit
`a62e02eea5ad61294e093721faec6f3b196b7676` (parent `81b82a0`). Its complete tree,
`da05cc2efd05243777d9611d6b34d402bb79eccd`, exactly matches published head
`4d0c255`; squash integration preserves the full published content and modes.

Local `main` was fast-forwarded to `a62e02e` and the implementation branch was
removed after that equality check. Archival/reference updates are on
`docs/archive-p04-08`, without a new commit or publication. Existing implementation,
functional/independent test and native review evidence remains unchanged.
The separate native review of the 75-line publication-notes commit was explicitly
skipped by the maintainer; this did not disable future reviews or undo the
implementation review. Live deployment prerequisites remain unverified.
