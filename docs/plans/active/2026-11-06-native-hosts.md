# Execution plan: Minimal native mobile hosts

- **Status:** Active
- **Date:** 2026-11-06
- **Roadmap task ID(s):** P10.02
- **Tracking issue / PR:** Not created; local feature-branch chain (no publication)
- **Owner:** Maintainer and implementation agents

## Goal and user-visible outcome

Compile minimal native Android Compose and iOS SwiftUI apps that display the
existing shared library marker. See the [task log](../../../odd/tasks/p10-02-native-hosts.md).

## Existing state and evidence

P10.01 (`b91d9b7`, records `8d45d93`) supplies the shared AAR and ARM64 device/
simulator frameworks. No application hosts exist. Local JBR 25 and Xcode 27.2
built all shared artifacts; Xcode remains outside Kotlin's published matrix.

## In scope

- Native hosts and build wiring; debug Android and unsigned iOS artifact checks.
- Record backend base URL `https://cuenora.carf.dev` for future client work.

## Out of scope

Common tests, UI system/navigation, HTTP/auth/financial functionality, backend
access, simulator/device launch, release signing, CI/E2E and publishing.

## Constraints and invariants

Reuse shared logic, keep UI native, pin dependencies and keep personal paths/
credentials out of projects. Only the iOS target's build-script sandbox may be
disabled as required by official KMP direct integration; never change global
security settings. No network permission or transport security exceptions needed.

## Steps

- [x] Android host and APK/lint checks.
- [x] iOS host and unsigned simulator/device builds.
- [x] Record verified work-unit commits, review state and roadmap update.

## Acceptance criteria

- [x] Both hosts compile and reference the shared marker rather than duplicating it.
- [x] Existing shared build still works; repository checks pass.
- [x] Configuration, backend URL and untested runtime boundaries are documented.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `:androidApp:assembleDebug :androidApp:lintDebug` | APK and lint pass | SDK 36 metadata failure; host compileSdk 37 correction passed, 0 lint errors / 3 warnings | Pass |
| `xcodebuild` Debug, simulator/device, signing disabled | Linked app for each SDK | Both passed; shared symbols in ARM64 apps, platform IDs 7/2 | Pass |
| Shared artifact tasks / independent probes | Preserved shared artifacts and host references | Own seam passed; independent candidate 72 tasks and baseline 22 tasks passed; AAR/header APIs unchanged | Pass |
| `python3 scripts/check_repository.py` | Pass | Independent candidate/baseline 544/532 files | Pass |

## Risks, assumptions, and decisions

Risk: items 2/5 (target-local script sandbox / build dependencies). Development
IDs and minimum deployment versions are provisional, not store/release policy.
No meaningful host RED exists before scaffold creation; real integration builds
and artifact probes verify consumption. Xcode local success is not official support.

## Completion notes

Hosts implemented and independently verified without implementation blockers.
Both unsigned Xcode builds passed in a clean candidate copy; S1/S2 passed,
S3 local branches preserve separate Android/iOS work units. Android keeps
ordinary debug signing with a disposable key under external `ANDROID_USER_HOME`;
no developer credentials were reused. APK inspection confirms invocation of the
shared getter. Android lint warnings (old target API, extraction rules, icon)
remain visible, not suppressed. iOS metadata extraction notes no AppIntents
dependency. No launch/runtime, network, release-signing or store-readiness proof.
Keep active until merge; no publishing authorized.
Rollback each host with its build wiring; shared library and backend stay intact.

Local chain: P10.01 `feat/p10-01-kmp-foundation` (`8d45d93`) → Android
`feat/p10-02-native-hosts` (`1341293`, 121 lines) → iOS
`feat/p10-02-ios-host` (`90f1753`, 331 lines) → integration records
`docs/p10-02-native-hosts`. Each future child PR targets its immediate parent;
no PR/tracker was opened because publishing is not authorized.

Native review mode was confirmed globally **off** during finalization. A preflight
attempt refused before mutation; no review was started and the switch was not
changed. Outcome: disabled/unmanaged, not approval. All functional and independent
checks above stand. Source/Debug build verification is complete; Release builds,
runtime launch, commonTest and backend connectivity remain untested/out of scope.
