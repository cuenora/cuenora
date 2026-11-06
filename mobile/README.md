# Shared mobile foundation

P10.01 introduces `:shared`: Kotlin common source compiled to an Android AAR and
static `CuenoraShared` frameworks for iOS device ARM64 and simulator ARM64.
There are no applications, screens, financial models or shared tests yet.
P10.02 adds hosts; P10.03 adds `commonTest`. Intel iOS simulators are not targeted.

## Configure and build

Open **this `mobile/` directory** in Android Studio. For terminal builds on macOS,
select the existing Android Studio JBR and your installed SDK for that shell:

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_HOME="$HOME/Library/Android/sdk"
cd mobile  # from the repository root
./gradlew --version
./gradlew :shared:assembleAndroidMain \
  :shared:linkDebugFrameworkIosArm64 \
  :shared:linkDebugFrameworkIosSimulatorArm64
```

Use your own installation paths when different. Do not commit `local.properties`
or machine-specific paths. No global Gradle installation is required. The first
build downloads the checksum-pinned Gradle distribution, plugins and Kotlin/Native
toolchain. Build outputs and caches are ignored by Git. iOS compilation requires
macOS with full Xcode selected, but does not require signing or a running simulator.
Android AAR compilation does not require an emulator.

## Pinned baseline

| Component | Selection |
| --- | --- |
| Kotlin Multiplatform | 2.4.21 |
| Android KMP library plugin | 9.3.1 |
| Gradle wrapper | 9.5.0 |
| Gradle runtime | Java 25 (local Android Studio JBR 25.0.3) |
| Android compile / minimum SDK | 36 / 23 |
| Android JVM bytecode | 17; separate from the JVM running Gradle |

Minimum SDK 23 is a provisional **library** baseline, not a promised app support
policy. Android/iOS application deployment targets remain a host-task decision.
No Compose, Ktor, serialization or other runtime library is added beyond Kotlin's
standard library. See [ADR-048](../docs/architecture/decisions/048-shared-mobile-build.md).

Kotlin's [published compatibility matrix](https://kotlinlang.org/docs/multiplatform/multiplatform-compatibility-guide.html)
lists Xcode 26.4 for this Kotlin version. Installed Xcode 27.2 is outside that
matrix: local compilation results do not establish official support. Do not
suppress compatibility checks. If an older Xcode is needed, install it separately
and select it per process using `DEVELOPER_DIR`, not a global selection change.

## Artifacts and next steps

Android output is under `shared/build/outputs/aar/`; Apple outputs are under
`shared/build/bin/iosArm64/debugFramework/` and
`shared/build/bin/iosSimulatorArm64/debugFramework/`.
Both contain the minimal `SharedModule.name` marker, not financial functionality.
On 2026-11-06, all three builds passed locally using JBR 25.0.3 and Xcode 27.2.
Inspection confirmed the marker in Android bytecode and both framework headers;
Apple archives identify iOS device/simulator separately with minimum iOS 15.0.
The [execution plan](../docs/plans/active/2026-11-06-kmp-foundation.md) records
observed commands, limitations and verification. This does not complete Phase 10
or prove either app can run. CI and E2E setup remain P10.12 and Phase 13.
