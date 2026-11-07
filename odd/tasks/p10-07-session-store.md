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

- T1 [S1] inline, verified: shared contract/codec, common regressions and ADR; commit pending. RED: 2/4 tests failed against stub codec; GREEN: 4/4 Android-host tests passed. Route: bounded understood shared foundation.
- T2 [S1] delegated, pending: Android Keystore encryption, no-backup atomic record and native regressions; commit pending. Writer reason: independent Android unit launched in parallel with T3, disjoint platform paths.
- T3 [S1] delegated, pending: iOS Keychain record and native regressions; commit pending. Writer reason: independent iOS unit launched in parallel with T2, disjoint platform paths.
- T4 [S1,S2] inline integration plus delegated verification, pending: native runtime proof, suite/artifacts, docs and completion; commit pending. Risk: items 1 (persisted credential format), 2 (secrets), 4 (ordering) and 5 (native test setup). Independent verifier required after own checks.

## Log

- L1 (2026-11-07): “Continuemos con la siguiente tarea P10.07”.
- L2 (2026-11-07): User approved the single-PR size exception with “Sí”.
- L3: Clean main `b5de97c` contains P10.06 (#128). Existing shared HTTP transport does not retain authorization. Backend policy rotates access/refresh together and requires local clear after logout; network orchestration is not part of this store.
- L4: Local Android SDK has adb, emulator and API37 ARM64 image. Do not reuse the existing personal AVD; native tests need isolated synthetic state. Xcode27.2/iOS26.5 are installed; Kotlin compatibility/debug-cache limitations persist. Real Keystore/Keychain execution must be observed, not inferred from JVM mocks.
- L5: T1 contract is SessionStore read()/replace(credentials)/clear(); only absent records return null. SessionCredentials redacts display and validates bounded opaque ASCII; SessionStoreException has redacted Corrupt/Unavailable reasons. SessionCodec and sessionOperation are the shared helpers both native units must reuse. Native operations share one process mutex on Dispatchers.IO. No guarantee against stale network refresh; orchestration remains out of scope.
