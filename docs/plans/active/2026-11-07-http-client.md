# Execution plan: Shared HTTP client and typed API errors

- **Status:** Completed locally; unmerged
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.06
- **Tracking issue / PR:** Issue waived by maintainer; single PR from `feat/p10-06-http-client` to `main`
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Add a shared Ktor JSON transport with typed failures, independently of the domain.
Original request: “Comencemos la tarea P10.06 del plan maestro docs/plans/master-plan.md”.

## Existing state and evidence

P10.04 domain and P10.05 explicit amount serializers exist. OpenAPI ApiError has
code, message, null details and nullable request_id. ADR-019 defines correlation;
Retry-After is integer seconds. Session storage and repositories remain P10.07–08.

## In scope

Ktor 3.6.1, Android/Darwin engines, injectable MockEngine, JSON requests/responses,
HTTP errors preserving unknown codes, malformed-response and transport failures,
bounded request timeout, cancellation propagation and explicit client lifecycle.

## Out of scope

Session storage, token refresh, financial repositories/DTOs, automatic retries,
UI wiring, remote smoke tests and shared Rust/Kotlin contract fixtures.

## Constraints and invariants

No domain changes, floating-point amounts, logging of payloads or credentials,
automatic redirects or application-level retries. Default origin is the previously
specified https://cuenora.carf.dev; tests use MockEngine only.

## Steps

- [x] Observe failing shared transport tests before implementing behavior.
- [x] Implement transport and document its boundary.
- [x] Run Android-host/iOS-simulator tests, allTests and three shared builds.
- [x] Independently verify new dependency integration and failure behavior.

## Acceptance criteria

Exact JSON survives requests/responses; 204 is supported; HTTP status, API code,
message, request ID and valid Retry-After survive failure mapping. Unknown codes,
non-JSON errors, network errors, timeouts and cancellation are distinguished.
Existing shared tests remain green. No backend request is performed.

## Verification

Use mobile README commands for both test targets, allTests and shared artifacts;
run python3 scripts/check_repository.py and git diff --check at root.
Observed: initial six transport tests failed against unimplemented behavior.
The implemented candidate passed 22 tests per Android-host/iOS-simulator target,
allTests and all three artifacts. Independent snapshots reproduced baseline 14
and candidate 22 passing tests per target, then 27 with five additional probes.
Those probes are now retained as regression tests. The final workspace run passed
all 27 tests per target (zero failures/errors/skips) and allTests.

## Risks, assumptions, and decisions

Risk: items 5 (new KMP runtime dependencies and native engines) and 2 (origin guard).
One focused, diff-resumable change; implementation inline, independent verification
after own checks. The maintainer subsequently authorized commit and PR publication
using the current gh session, with no issue and a single-PR size exception.
Native review is disabled globally. Existing Xcode/debug-cache warnings may remain.

## Completion notes

No independent behavioral blockers. Probes verified the actual 30-second timeout,
caller cancellation, per-request credential nonretention, correlation precedence,
query origin preservation and large exact numeric text. All ten pre-existing
Kotlin source/test files were byte-identical to baseline. Android AAR and both
frameworks retain new transport and existing marker exports. Older README claims
that HTTP was still absent were corrected.

The JSON tree parser accepts certain unquoted invalid literals even with strict
defaults. Transport now validates literal grammar recursively, without numeric
conversion; HTML and nested NaN regressions cover this. The deprecated Ktor I/O
exception alias was replaced by kotlinx.io.IOException before final builds.

Repository validation and git diff --check passed. No backend network, physical
device, UI/E2E, CI or real TLS/DNS check was performed. Existing Xcode/debug-cache
warnings remain nonfatal. Native review was not started (disabled globally).
At initial verification, no commit, push or PR had been requested. Verification
scratch was removed by the parent.

### Follow-up: single backend configuration

Maintainer request: “Podemos hacer que si luego necesitamos cambiar la url del backend que ahora es https://cuenora.carf.dev mientras desarrollamos pero luego cuando tengamos el backen de produccion será otra, se pueda cambiar en un unico sitio facilmente?”

Extracted the unchanged default into shared `config/BackendConfig.kt`, consumed by
ApiClient on both targets; explicit constructor overrides remain available. Future
changes require editing BASE_URL once and rebuilding both apps. This is a constant
extraction with no changed transport behavior, so no meaningful behavioral RED is
claimed. Added a default-origin regression; existing custom-origin coverage remains.
Follow-up checks passed: 28 tests per Android-host/iOS-simulator target (zero
failures/errors/skips), allTests, all three artifacts, repository validation and
git diff --check. The development hostname occurs in exactly one Kotlin source
location, BackendConfig.BASE_URL. No remote requests or publishing performed.
Risk: none (configuration extraction only).
