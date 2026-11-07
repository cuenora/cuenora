# Execution plan: shared mobile tests in CI

- **Status:** Completed; hosted shared-test jobs and reports verified
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.12
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Run shared Android-host and iOS ARM64 simulator tests in GitHub Actions, with
JUnit evidence proving tests executed. Branch: `ci/p10-12-shared-mobile-tests`.
The separate P10.11 worktree remains untouched; this branch starts at `e6a8fba`.

## Existing state and evidence

CI currently checks repository conventions, Rust and dependencies on Ubuntu.
The shared test commands already exist, with 73 Android-host and 74 iOS tests
on this baseline. A skipped Gradle task is not evidence of executed tests.

## In scope

- Separate matrix entries for Ubuntu Android-host and macOS ARM iOS simulator.
- Java 25, existing wrapper, SDK prerequisites and per-process Xcode selection.
- Fail-closed JUnit validation, retained reports, local tests and documentation.

## Out of scope

App/device/E2E tests, signing, releases, backend changes, dependency upgrades,
P10.11 changes, publishing and remote execution without explicit authorization.

## Constraints and invariants

Preserve existing jobs, triggers and read-only token permissions. No secrets,
persisted checkout credentials, write caches from PRs or suppressed test failures.
Pin the runner family and select Xcode 26.4.1, matching Kotlin's documented 26.4
baseline. Do not change global Xcode selection.

## Steps and acceptance criteria

- [x] Observe failing report-validation tests, implement validator and CI matrix.
- [x] Run validator tests, workflow lint and local shared tests on both targets.
- [x] Independently verify CI/report failure boundaries (Risk item 5: CI).
- [x] Observe both jobs in hosted CI before checking P10.12 complete.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 tests/infrastructure/test_mobile_test_results.py -v` | Missing/empty/broken/failed/skipped reports fail | Initial missing-script RED; 4 groups GREEN | Passed |
| `actionlint .github/workflows/ci.yml` (1.7.12); repository/diff checks | Pass | All passed | Passed |
| `./gradlew --no-daemon --console=plain :shared:testAndroidHostTest :shared:iosSimulatorArm64Test` | Target tests execute | 73 Android-host / 74 iOS-simulator, no failures/errors/skips | Passed locally |
| `python3 scripts/check_mobile_test_results.py <target-report-directory>` | Nonzero tests, no failures/errors/skips | Both actual report directories accepted | Passed |
| Independent CI verification | No change-caused blocker | Four regression groups, multi-suite success, 14 rejection probes and exact workflow shell probes passed | Passed locally |
| GitHub hosted Android/iOS jobs | Both pass with report artifacts | Run `42426340885`: both passed; downloaded XML verifies 76 Android / 77 iOS, no failures/errors/skips | Passed |

## Risks, assumptions, and decisions

Risk item 5: CI configuration. Local Xcode 27.2 is outside Kotlin's compatibility
matrix; local success cannot prove the hosted Xcode 26.4.1 environment.
Runner/action selections use their upstream documentation; see mobile CI guide.

## Completion notes

The maintainer authorized commit, publication and a PR without issue linkage.
Keep P10.12 unchecked until hosted execution provides the missing evidence.
Documentation edits are separated from P10.11 so both PRs can target `main`.
Local target execution used documented JBR 25/Android SDK and installed Xcode
27.2; existing non-fatal Native debug-cache warnings remain. The hosted Xcode
26.4.1 image was checked against upstream inventory, not executed locally.
No Rust or production mobile source changed; backend tests are not applicable.
Independent verification used an external fresh baseline/candidate snapshot.
Parsed existing jobs/triggers/permissions/concurrency matched baseline. SDK setup,
task quoting, Gradle failure propagation, target report paths, ARM64 rejection
and per-process Xcode selection passed stubbed shell probes. Rejected report
commands left input hashes unchanged. The archive-only verifier could not run
the Git-dependent repository checker; the parent reran it in the real worktree.
Hosted compatibility and report uploads remain unproven, not silently passed.

### 2026-11-08: correct the published Android platform package ID

Both new jobs in hosted run `42422485455` failed during SDK provisioning:
`Failed to find package 'platforms;android-37'`. Google's official
[SDK repository catalog](https://dl.google.com/android/repository/repository2-3.xml)
publishes `platforms;android-37.0`, not the requested ID. Prior shell probes
checked argument forwarding but failed to validate the published package names.
This was a defect in the new workflow, not evidence that the runner was broken.

Corrected the one shared matrix step. Added
`MobileSdkPackagesTest.test_workflow_requests_published_sdk_package_ids`, which
executes that exact workflow block against a small offline catalog of the
verified published IDs, including an SDK path containing spaces. Observed RED
on the original workflow (`Failed to find package: platforms;android-37`) and
GREEN after correction. The regression does not replace fresh hosted proof.
All five local CI regression groups and actionlint 1.7.12 passed. An independent
fresh snapshot verified catalog membership, original-workflow RED/corrected GREEN
and that the only workflow delta is the package identifier; no check was weakened.

Hosted [run 42426340885](https://github.com/cuenora/cuenora/actions/runs/42426340885)
on correction `1ff17d2` passed both shared-test jobs, including SDK provisioning,
Xcode selection, test execution, report validation and upload. Downloaded both
report artifacts and reran the public validator: 76 Android-host and 77 iOS tests,
zero failures/errors/skips. Earlier pending-hosted notes above describe the
initial delivery; this run supplies the missing proof. Synchronized merged
P10.11 from `main` without conflicts before recording completion.
