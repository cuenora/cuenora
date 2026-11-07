# P10.07: Native secure session storage
Branch: `feat/p10-07-session-store`; baseline: `b5de97c`. Delivery: `exception-ok`, one PR with maintainer-approved size exception; forecast ~900 authored lines.
Runner: shared Android-host/iOS-simulator tests, isolated Android instrumentation, shared artifacts and repository checks. No remote backend operation.

## Specs

- S1: “Continuemos con la siguiente tarea P10.07”. Requested roadmap item: “Define `SessionStore` with native Keychain/Keystore implementations.” Acceptance: shared read/replace/clear contract and actual native storage implementations, with tests and documentation. Session orchestration and repositories remain out of scope.
- S2: “Sí”, answering “¿Autorizás también para esta tarea una única PR con excepción de tamaño, manteniendo código, pruebas y documentación juntos?”. One cohesive eventual PR; this does not authorize publishing yet.

Implementation choices: store the access/refresh pair in one versioned record;
redact credential display and storage exceptions; distinguish absence from failure;
no plaintext fallback, backup/sync or automatic auth/network behavior. Native stores
share an in-process mutex and operate off the caller thread. Namespace defaults to
BackendConfig.BASE_URL, isolating credentials when the backend changes. No claim of
cross-process transactions, memory zeroization, biometric gates or hardware backing
on all devices. Native failure/cancellation can leave an operation's outcome ambiguous.

## Tasks

- T1 [S1] inline, verified: shared contract/codec, common regressions and ADR; commit `8bba834` plus explicit source-set resolution fix. RED: 2/4 tests failed against stub codec; GREEN: 4/4 Android-host tests passed. Route: bounded understood shared foundation.
- T2 [S1] delegated, verified: Android Keystore encryption, no-backup atomic record and five native regressions; independent audit found no reproducible source blocker; commit pending. Writer reason: independent Android unit launched in parallel with T3, disjoint platform paths.
- T3 [S1] delegated, partial: iOS Keychain record and six native regressions compile; runtime unavailable (-25291), commit pending. Writer reason: independent iOS unit launched in parallel with T2, disjoint platform paths.
- T4 [S1,S2] inline integration plus delegated verification, needs decision: independent audit/build checks passed, but native iOS proof remains unavailable. Risk: items 1 (persisted credential format), 2 (secrets), 4 (ordering) and 5 (native test setup).

## Log

- L1 (2026-11-07): “Continuemos con la siguiente tarea P10.07”.
- L2 (2026-11-07): User approved the single-PR size exception with “Sí”.
- L3: Clean main `b5de97c` contains P10.06 (#128). Existing shared HTTP transport does not retain authorization. Backend policy rotates access/refresh together and requires local clear after logout; network orchestration is not part of this store.
- L4: Local Android SDK has adb, emulator and API37 ARM64 image. Do not reuse the existing personal AVD; native tests need isolated synthetic state. Xcode27.2/iOS26.5 are installed; Kotlin compatibility/debug-cache limitations persist. Real Keystore/Keychain execution must be observed, not inferred from JVM mocks.
- L5: T1 contract is SessionStore read()/replace(credentials)/clear(); only absent records return null. SessionCredentials redacts display and validates bounded opaque ASCII; SessionStoreException has redacted Corrupt/Unavailable reasons. SessionCodec and sessionOperation are the shared helpers both native units must reuse. Native operations share one process mutex on Dispatchers.IO. No guarantee against stale network refresh; orchestration remains out of scope.
- L6: Enabling instrumented tests requires getByName("androidDeviceTest") in this build script; an accessor form failed Gradle script compilation. Explicit resolution fixed the setup and the four common tests passed again. Risk assessment remains pending for delegated native units; RDD mode is off.
- L7: Kotlin/Native requires CancellationException in suspend @Throws; Dispatchers.IO is internal in the pinned Native API. Parent corrected @Throws and uses Dispatchers.Default for off-caller-thread bounded native operations. The shared mutex remains process-wide. The explicit source-set fix is commit 505b782.
- L8: Android writer observed meaningful instrumentation RED against stubs, then 5/5 GREEN on isolated API37 emulator-5580; assembleAndroidDeviceTest passed. Emulator stopped. iOS source/tests compile; normal runtime ran 38 tests, all six Keychain tests failed. Scratch-only diagnostic identified SecItemDelete -25291 (errSecNotAvailable), not an established entitlement root cause. No access-control bypass or plaintext fallback was added. Native iOS proof remains missing; do not check P10.07 complete.
- L9: Independent verifier returned PARTIAL for S1 (missing native iOS proof), S2 scope satisfied. Baseline 28/28 host tests; candidate 32/32 plus test APK, AAR and both frameworks built. Existing 28 test identities/outcomes match. No reproducible source blocker. Native failed-write preservation probes were not executed; do not claim this proof. CF cast warnings remain advisory. Parent confirmed Android GREEN scratch used the current shared helper, and corrected a stale dispatcher KDoc.
- L10: Needs decision: whether to add app-hosted iOS tests to investigate/validate Keychain. No further runtime retry, security bypass, publishing or master-plan completion. Existing Native test failures remain visible, not skipped. Private emulator and verification scratch must be cleaned after preserving this evidence.
