# Execution plan: Native secure session storage

- **Status:** Needs decision: iOS native runtime proof unavailable
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.07
- **Tracking issue / PR:** Not created
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
- [ ] iOS Keychain implementation and native tests.
- [ ] Shared tests, artifacts, native checks and independent verification.

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
Native Keychain behavior and failed-native-write invariants remain unproven.
No further simulator retry or access-control bypass was performed. allTests is
not claimed passing. Repository validation and git diff --check passed.

## Risks, assumptions, and decisions

Risk: items 1, 2, 4 and 5 (credential format/security, ordering and native tests).
Maintainer approved one PR with a size exception. Native review remains disabled
unless the user changes its switch. New shared and native code needs independent
security/failure verification. Native access-control or emulator limitations must
be reported as unavailable proof, never silently bypassed.

## Completion notes

Independent audit found no reproducible source blocker, but returned PARTIAL for
missing iOS native proof. The master-plan checkbox remains open. Proposed next
step: investigate app-hosted iOS tests rather than treating a CLI test runner's
Keychain failure as success; target changes require the maintainer's decision.

Known nonfatal Xcode/debug-cache warnings persist, plus CF-pointer cast warnings
in the new iOS bridge. No suppressions were added. No physical device, backup/
restore, process-crash fault injection or network authentication proof is claimed.
No remote calls, push or PR. Native review is disabled globally; delegated-diff
assessment classified security risk high and triggered independent verification,
not a review transaction. See the task log for commits and pending state.
