# Mobile foundation and native hosts

P10.01 introduces `:shared`: Kotlin common source compiled to an Android AAR and
static `CuenoraShared` frameworks for iOS device ARM64 and simulator ARM64.
P10.02 adds minimal [Android Compose](androidApp/README.md) and
[iOS SwiftUI](iosApp/README.md) hosts displaying the shared marker. These are
integration shells, not financial screens. P10.03 adds `commonTest`.
Intel iOS simulators are not targeted.

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

Build the Android host from `mobile/`:

```bash
./gradlew --no-daemon :androidApp:assembleDebug :androidApp:lintDebug
```

Open `iosApp/Cuenora.xcodeproj` for the iOS host; its [instructions](iosApp/README.md)
include unsigned command-line builds and the Java environment required by its
pre-Sources Gradle phase. For automated Android checks, use an external temporary
`ANDROID_USER_HOME` to isolate the generated debug key. No release-signing
configuration is checked in.

## Backend base URL

The maintainer-provided backend origin is **`https://cuenora.carf.dev`**.
Future client work must use the versioned API contract under `/v1`; P10.02 does
not add a client, send requests, verify server availability or grant remote
administration access. HTTP integration remains P10.06.

## Pinned baseline

| Component | Selection |
| --- | --- |
| Kotlin Multiplatform | 2.4.21 |
| Android KMP library plugin | 9.3.1 |
| Gradle wrapper | 9.5.0 |
| Gradle runtime | Java 25 (local Android Studio JBR 25.0.3) |
| Android compile / minimum SDK | 36 / 23 |
| Android host compile / target / minimum SDK | 37 / 36 / 23 |
| Android JVM bytecode | 17; separate from the JVM running Gradle |

Minimum Android SDK 23 and iOS 15 are provisional development baselines, not a
promised app support policy. Compose is confined to the Android host; the shared
module still adds no client/runtime libraries beyond Kotlin's standard library.
See [ADR-048](../docs/architecture/decisions/048-shared-mobile-build.md) and
[ADR-049](../docs/architecture/decisions/049-native-host-integration.md).

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
or prove runtime UI behavior. The [P10.02 execution record](../docs/plans/active/2026-11-06-native-hosts.md)
tracks host verification separately. CI and E2E setup remain P10.12 and Phase 13.
