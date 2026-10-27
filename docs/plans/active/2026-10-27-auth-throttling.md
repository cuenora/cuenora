# Execution plan: bounded authentication throttling

- **Status:** Active — implementation and local verification complete; size exception approved, commit/native review pending
- **Date:** 2026-10-27
- **Roadmap task ID(s):** P04.08
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Bound registration, login and refresh attempts before expensive work, with safe
429 responses and explicit trusted-client identity for Cloudflare → Traefik → API.
See the specification and log in [the feature document](../../../odd/tasks/p04-08-auth-throttling.md).

## Existing state and evidence

The API has 8 KiB JSON limits, password/refresh concurrency admission and typed
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
- [ ] 5. Resolve delivery-size strategy, commit and follow native review if applicable.

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

The maintainer approved a single oversized PR without publication. Work-unit
commit and native candidate review remain pending; no publication or remote operation is authorized. The native review
switch is on; the preliminary assessment failed closed as high/due because
untracked paths need an explicit inventory declaration. No review transaction
was started and no approval is claimed. Reassess the selected committed unit
with the canonical inventory after its boundary is approved. Do not archive or
mark the roadmap complete before closure. The full 46-case suite preceded the
final test-only fresh-email strengthening; production code did not change.

Rollback boundary: revert the single authentication-protection unit with its
config, errors, HTTP regressions and documentation. Existing schemas, stored
identities, sessions and financial records are unchanged; no migration or
persistent counter cleanup is needed.
