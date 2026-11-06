# Execution plan: Shared KMP foundation

- **Status:** Active
- **Date:** 2026-11-06
- **Roadmap task ID(s):** P10.01
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Create the independently buildable `mobile/shared` Kotlin Multiplatform library
with Android, iOS device ARM64 and iOS simulator ARM64 targets.

## Existing state and evidence

ADR-001 approves KMP shared logic with native Compose/SwiftUI hosts. No mobile
build exists at baseline `a125116`. Android Studio, SDK and bundled JBR exist;
Xcode 27.2 is selected. See the [logbook](../../../odd/tasks/p10-01-kmp-foundation.md).

## In scope

- Pinned Gradle wrapper and plugins, shared source and target artifacts.
- Local configuration, compatibility documentation and scoped verification.

## Out of scope

Hosts (P10.02/P12.01), commonTest (P10.03), UI, financial models, API clients,
CI/E2E, signing, distribution and remote operations.

## Constraints and invariants

Keep backend independent and financial behavior unchanged. No global toolchain
changes, personal paths or credentials in build files. Preserve native UI boundaries.

## Steps

- [x] Create the shared library and reproducible wrapper.
- [x] Compile and inspect Android/iOS artifacts; record unsupported-toolchain issues.
- [x] Run independent verification and update scoped documentation/status.

## Acceptance criteria

- [x] Shared source compiles to Android AAR and both ARM64 iOS frameworks.
- [x] Wrapper integrity, artifact contents and repository checks pass.
- [x] Instructions distinguish this library from hosts and shared tests.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `./gradlew --no-daemon --version` in `mobile/` | Pinned Gradle/JVM | Gradle 9.5.0, JBR 25.0.3; parent repeated | Pass |
| `./gradlew --no-daemon :shared:tasks --all` | Target tasks exist | All three present | Pass |
| `./gradlew --no-daemon :shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64` | AAR and two frameworks | 22 tasks executed, exit 0 | Pass |
| Artifact inspection | Shared symbol and expected architectures | Marker in AAR and both headers; ARM64 static archives with device/simulator platform IDs | Pass |
| Independent verification | S1–S2 independently verified | Both pass in clean scratch; 22 build tasks executed; artifacts and official checksums verified | Pass |
| `python3 scripts/check_repository.py` | Pass | Candidate 532 and baseline 520 Git-visible files | Pass |

## Risks, assumptions, and decisions

Risk: item 5 (build/dependencies). Published Kotlin matrix does not list installed
Xcode 27.2. Do not suppress its warning or call local success official support.
No meaningful runnable RED exists before the build; compilation is the scaffold
acceptance check. Common tests remain P10.03.

## Completion notes

Wrapper JAR SHA-256 `497c8c2a7e5031f6aa847f88104aa80a93532ec32ee17bdb8d1d2f67a194a9c7`
matches the official Gradle distribution checksum endpoint. No compatibility
warnings were suppressed; local Xcode 27.2 compilation passed, without establishing
official support. Kotlin/Native bundle setup warned about overwriting an existing
nonempty directory inside isolated scratch; no global toolchain was modified.
Independent verification passed S1–S2 with no blockers. Backend/contracts/infra/
scripts/CI bytes are identical to baseline; backend suites were not rerun.
Hosts, shared tests and device execution were deliberately not attempted.
Implementation is committed as `b91d9b7` on `feat/p10-01-kmp-foundation`.
Native assessment returned high risk (executable wrapper / build dependencies).
On 2026-11-06 the maintainer skipped this candidate's native review; the exact
decline returned `declined_this_candidate` for the matching target. No native
review record or approval was created; future reviews remain enabled.
Implementation and scoped verification are complete; publication remains pending.
Rollback boundary: new mobile build, ADR and linked documentation only;
no backend or database changes. The plan remains active until merge.
