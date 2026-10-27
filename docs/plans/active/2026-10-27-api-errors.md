# Execution plan: typed API errors and request IDs

- **Status:** Active
- **Date:** 2026-10-27
- **Roadmap task ID(s):** P04.07; P04.06 explicitly deferred
- **Tracking issue / PR:** Not created
- **Owner:** Agent with maintainer decisions

## Goal and user-visible outcome

Return a consistent typed error envelope and a server-generated request ID without changing existing success bodies, error codes, statuses or security headers. The maintainer approved replacing the legacy error envelope.

## Existing state and evidence

`registration_http` serves liveness, register, login and refresh; `auth_identity` supplies reusable route-scoped authentication. Errors use exact `{error: ...}` JSON, while the master plan proposes `code`, `message`, `details`, `request_id`. Financial endpoints do not exist, so ownership enforcement remains P04.06 until resource operations are introduced.

## In scope

- One shared typed error representation, safe static messages and null details.
- Server-generated diagnostic IDs in error JSON and `X-Request-ID`, plus a typed request extension.
- Consistent router 404/405 errors and explicit final-router context integration.
- Contract, setup, regression tests and architectural decision record.

## Out of scope

Financial endpoints, ownership scaffolding, rate limits, logging infrastructure, global revocation, verification/recovery, dependencies and API path versioning.

## Constraints and invariants

Preserve statuses, stable codes, security headers, success payloads, exact credential handling and database behavior. Never reflect supplied request IDs, credentials or backend diagnostics. A request ID is not an idempotency key or authorization proof.

## Steps

- [x] 1. Observe new public-interface tests fail before source implementation.
- [x] 2. Implement the shared contract/context and adapt existing error tests/docs.
- [x] 3. Run focused, workspace, isolated PostgreSQL and independent checks.
- [ ] 4. Resolve delivery size strategy, commit and run applicable native review with separate candidate consent.

## Acceptance criteria

- [x] Every application error has the approved four keys; body/header IDs agree when generation succeeds.
- [x] Success bodies, codes, statuses, ownership/auth behavior and security headers are preserved.
- [x] Server IDs ignore untrusted client hints and survive nested context without regeneration.
- [x] Entropy failure rejects before handler with safe 503 and explicit absent ID.
- [x] OpenAPI, setup and regression tests describe the same behavior.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `cargo test -p cuenora-api --test api_errors --locked` | New contract/ID tests pass after observed RED | Passed; see completion notes | Pass |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features --locked` | All pass | Passed; see completion notes | Pass |
| Disposable registration/login/refresh/identity HTTP groups | Actual DB cases run and preserve rejection snapshots/session behavior | Passed; see completion notes | Pass |
| Independent isolated base/candidate HTTP probes | Only approved envelope/ID and normalized404/405 changes | Passed; see completion notes | Pass |
| `python3 scripts/check_repository.py`; `git diff --check` | Pass | Passed; see completion notes | Pass |

## Risks, assumptions, and decisions

Risk: items 2/3, authentication response integration and shared public error contract. No token validation, storage query, permission or persisted format changes intended. Four-field error replacement explicitly approved; static English messages, null details and server-generated 128-bit IDs selected for privacy and simplicity. If OS randomness fails, return 503 before handler with null request_id and no ID header. The maintainer accepted one coherent oversized implementation commit and future PR (`exception-ok`, about 1,450 authored lines), explicitly without publication. Code, tests and OpenAPI must migrate together; native review consent remains separate.

## Completion notes

Implementation and own checks passed; independent verification also passed; the size exception is approved; local commit and native review remain pending. Observed RED 2/2 for missing ID, then GREEN 5/5 public HTTP tests. Fmt/Clippy passed; workspace 58 passed including 11 doctests, 31 DB tests intentionally ignored by ordinary Cargo. Disposable HTTP harness registration 4/login 5/identity 3 passed; refresh initially 4/5 with one stale legacy assertion, corrected and refresh-only rerun 5/5. Repository validation 203 files and local OpenAPI reference validation passed. Independent S1–S4 passed: 126 differential HTTP cases, 27 HEAD responses, 3 isolated identity DB groups and 63 OpenAPI local references; parent reran the differential comparison successfully. New verifier probes for non-UTF8 duplicate hints and HEAD entropy failure were pinned and passed. Separate external Cargo targets were required after stale shared-target linkage; candidate reruns passed. No token/store/DB changes or dependencies. Detailed spec and live evidence: `odd/tasks/p04-07-api-errors.md`. No push/PR authorized. Move this record only after verified merge.
