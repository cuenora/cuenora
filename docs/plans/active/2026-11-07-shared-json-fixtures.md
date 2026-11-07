# Execution plan: shared JSON contract fixtures

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.11
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Test the same checked-in JSON response fixtures in Rust and Kotlin Multiplatform.
No production behavior or public contract changes.

## Existing state and evidence

Rust account/transaction listing types serialize the backend's responses. KMP
repositories parse independent inline test payloads. Neither side currently reads
a shared fixture corpus. Native hosts remain marker-only; P10.10 is deferred.

## In scope

- Account listing with exact signed balances, metadata and pagination.
- Accountless income listing with explicit nullable fields.
- Cross-currency transfer listing with flattened quote and comparison metadata.
- Dependency-free, test-only fixture embedding for both KMP targets.

## Out of scope

All-endpoint coverage, HTTP/database integration, production changes, UI and CI.
Existing isolated PostgreSQL route tests remain separate evidence.

## Constraints and invariants

Use synthetic records and exact decimal strings. Rust must exercise existing
response serializers and transaction row conversion, not duplicate wire DTOs.
KMP must exercise public HTTP repositories. One JSON corpus is authoritative;
generated Kotlin belongs only under ignored build output and commonTest.

## Steps

- [x] Add Rust serializer assertions and observe the missing-corpus failure.
- [x] Add the shared corpus, KMP embedding and repository assertions.
- [x] Run focused and suite checks, then independently verify build isolation.
- [x] Record evidence and update P10.11 only after verification.

## Acceptance criteria

- [x] Both languages consume the same fixture bytes without committed copies.
- [x] Actual Rust output matches complete JSON objects, including null/absent fields.
- [x] Android-host and iOS-simulator repository tests preserve exact values.
- [x] Production artifacts remain free of generated test fixtures.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `cargo test -p cuenora-api --lib shared_json --locked` | Two fixture tests pass | RED: two missing-file failures; GREEN: 2 passed | Passed |
| `cargo test --workspace --all-features --locked` | Pass; DB tests separate | 188 passed, 251 ignored, zero failures | Passed |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass | Both passed | Passed |
| `:shared:testAndroidHostTest`; `:shared:iosSimulatorArm64Test`; `:shared:allTests` | Executed tests pass | 76 Android-host / 77 iOS-simulator, zero failures/errors/skips | Passed |
| `:shared:assembleAndroidMain`; `:shared:linkDebugFrameworkIosArm64`; `:shared:linkDebugFrameworkIosSimulatorArm64` | Build | All three passed | Passed |
| `python3 scripts/check_repository.py`; `git diff --check` | Pass | Both passed | Passed |
| Independent generation/isolation probes | Pass | Rust 2, Android focused 3, iOS 77 passed; regeneration and artifact isolation confirmed | Passed |

## Risks, assumptions, and decisions

Test-only Gradle source generation touches build configuration (Risk item 5).
Use declared input/output dependencies; verify regeneration and test-only scope.
No new dependencies or runtime configuration. RDD is disabled globally.

## Completion notes

Focused Android execution ran the three new common tests before the full suite.
The full KMP invocation used `./gradlew --no-daemon` and the documented JBR/SDK.
Existing Kotlin/Native missing debug-cache object warnings remain non-fatal.
Rust's initial test compilation exposed incorrect currency variant capitalization
in the new test; this was corrected before the observed missing-file RED.
No production behavior changed, so no runtime-behavior RED or CLI differential
check applies. PostgreSQL tests were not run; these checks are DB-free serializers.
Independent verification used a fresh system-temp snapshot of baseline
`e6a8fba` plus this candidate. Generated Kotlin decoded byte-for-byte to JSON;
an unchanged rerun was up-to-date. Changing `Daily` to `Changed` reran generation
and compilation and failed exactly the account test; restoring it returned green.
The AAR and simulator framework contained no fixture symbols/payload. All mobile
production sources and preexisting Rust production bytes matched the baseline.
Parent reran the focused Rust command after the independent check.
The maintainer authorized commit, publication and a PR without issue linkage.
P10.12 is a separate PR; P10.10 stays deferred.
