# Execution plan: Native secure session storage

- **Status:** Completed locally; unmerged
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.07
- **Tracking issue / PR:** Issue waived by maintainer; [PR #129](https://github.com/cuenora/cuenora/pull/129)
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Provide shared SessionStore read/replace/clear operations backed by Android
Keystore encryption and iOS Keychain. See the [task log](../../../odd/tasks/p10-07-session-store.md).

## Existing state and evidence

ADR-052 leaves session storage separate from HTTP. Identity policy rotates the
access/refresh pair together. Mobile hosts remain integration shells. Native
tools exist locally, but runtime secure-storage tests have not yet been observed.

## In scope

Shared credential/storage contract, redacted errors, a single versioned pair,
native secure persistence, native/common regression tests and integration proof.

## Out of scope

Login/logout/refresh orchestration, repositories, UI, biometric unlock policy,
cross-process access, remote backend calls and mobile CI rollout.

## Constraints and invariants

No plaintext fallback, secret logs or synchronized/backed-up credentials. Native
storage absence must not hide read errors. Backend namespaces remain isolated.
Use synthetic credentials and isolated native test state only.

## Steps

- [x] Shared contract and deterministic codec tests (observed RED/GREEN).
- [x] Android Keystore-backed implementation and native tests.
- [x] iOS Keychain implementation and app-hosted native tests.
- [x] Shared tests, artifacts, native checks and independent verification.

## Acceptance criteria

Read, replace and clear a complete pair across fresh store instances. Reject
corruption without returning partial credentials. Native tests verify actual
Keystore/Keychain operations; common/JVM tests alone are insufficient.

## Verification

Common codec: four Android-host tests, initially two failures against stubs, then
all four passing. Full independent host baseline: 28 tests; candidate: 32 passing.
Android instrumentation: five passing tests on an isolated API37 AVD, following
meaningful RED against a stub. `:shared:assembleAndroidDeviceTest` passed.

Independent command from mobile with the documented JBR/SDK environment passed:
`:shared:testAndroidHostTest :shared:assembleAndroidDeviceTest :shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64`.
All 28 existing test identities/outcomes were unchanged. Swift headers export both
SessionStoreException and CancellationException as errors.

Normal iOS runtime attempt: 38 tests, six Keychain failures, 32 common passes.
Scratch-only diagnostics found SecItemDelete -25291 (errSecNotAvailable). This is
not an established entitlement/root-cause diagnosis. A diagnostic passthrough also
broke one redaction test in scratch only; that diagnostic was never copied here.
After the maintainer authorized app-hosted tests, six XCTest cases exposed a
second, concrete read-query bug: CFNumber instead of required CFBoolean flags
caused errSecParam (-50). Inserting exact CF constants corrected it; NSNumber
construction alone did not. All six app-hosted cases then passed with local ad-hoc
signing and no team/custom entitlement. nm showed the Kotlin implementation in
the app only, not duplicated in the test bundle.

The six CLI Keychain cases still failed after that fix, so their public-API
coverage moved to the app-hosted suite. The pure Native CFBoolean test remains
under iosTest. No case was ignored. Final own verification passed 32 Android-host
and 33 iOS-simulator tests, allTests, Android AAR and both frameworks, followed by
all six app-hosted cases. Native failed-write fault injection is still not claimed.
Repository validation and git diff --check passed.

## Risks, assumptions, and decisions

Risk: items 1, 2, 4 and 5 (credential format/security, ordering and native tests).
Maintainer approved one PR with a size exception. Native review remains disabled
unless the user changes its switch. New shared and native code needs independent
security/failure verification. Native access-control or emulator limitations must
be reported as unavailable proof, never silently bypassed.

## Completion notes

The original independent audit returned PARTIAL for missing iOS native proof.
The targeted independent recheck now passed all S1–S3: 32/32 host, 33/33 Native,
allTests, AAR/two frameworks and 6/6 app-hosted tests on a fresh isolated simulator.
No reproducible blocker remained. Both simulator bundles are locally ad-hoc
signed with no TeamIdentifier; the test bundle reuses the host's Kotlin runtime.
The previously missing proof is closed. See the task log for commit evidence and
the complete test migration map.

Parent workspace spot-check independently reran the complete Gradle command:
32 host/33 Native, allTests and all three artifacts passed. The existing unsigned
generic-device Xcode Debug build also passed: ARM64 app, bundle org.cuenora.ios,
minimum iOS15, no XCTest product built for the generic device target.

Known nonfatal Xcode/debug-cache warnings persist, plus CF-pointer cast warnings
in the new iOS bridge. No suppressions were added. No physical device, backup/
restore, process-crash fault injection or network authentication proof is claimed.
No remote calls, push or PR. Native review is disabled globally; delegated-diff
assessment classified security risk high and triggered independent verification,
not a review transaction. See the task log for commits and pending state.
