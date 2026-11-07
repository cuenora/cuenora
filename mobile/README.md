# Mobile foundation and native hosts

P10.01 introduces `:shared`: Kotlin common source compiled to an Android AAR and
static `CuenoraShared` frameworks for iOS device ARM64 and simulator ARM64.
P10.02 adds minimal [Android Compose](androidApp/README.md) and
[iOS SwiftUI](iosApp/README.md) hosts displaying the shared marker. These are
integration shells, not financial screens. P10.03 adds shared `commonTest` tests.
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

## Shared unit tests

From `mobile/`, with the Java and Android SDK environment above:

```bash
./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test
```

Both tasks run `SharedModuleTest` from `shared/src/commonTest/kotlin/`, asserting
the existing `SharedModule.name` marker. Android uses local JVM host tests,
explicitly enabled with `withHostTest {}`; no Android emulator is needed.
iOS uses the Kotlin/Native ARM64 simulator test runner and requires macOS/Xcode
with an installed iOS simulator runtime. These are shared-library unit tests,
not device, UI, backend or end-to-end tests.

Run the shared verification lifecycle with `./gradlew --no-daemon :shared:allTests`.
Check the per-target XML reports under `shared/build/test-results/`: a successful
build or a skipped device-target task is not proof that tests actually ran.
`iosArm64` device execution is not part of this local simulator-based check.
CI automation remains P10.12.

On 2026-11-07 the marker test passed once per target, with zero failures, errors
or skipped cases, both locally and in an independent clean snapshot. All three
shared artifact builds also passed; Android marker bytecode and both iOS exported
headers matched the baseline. Native linking emitted non-fatal missing debug-cache
object warnings; the Xcode compatibility limitation below still applies.

If a build restored from another toolchain location fails with duplicate native
KLIB names, run `./gradlew --no-daemon :shared:clean` and repeat the tests before
considering configuration changes. Do not suppress Kotlin/Native diagnostics.

See the [Android KMP test configuration](https://developer.android.com/kotlin/multiplatform/plugin)
and [Kotlin shared-test guide](https://kotlinlang.org/docs/multiplatform/multiplatform-run-tests.html).

## Shared financial domain

P10.04 adds `Money`, `Account`, `Transaction` and `SavingsForecast` under
`org.cuenora.shared.domain`; see [ADR-050](../docs/architecture/decisions/050-shared-financial-domain-models.md)
for their invariants and boundaries. Recorded money uses signed `Long` hundredths
with explicit currency. Forecast rationals/projected minor units retain arbitrary
precision separately, without arithmetic or conversion to recorded balances.
Models use typed opaque identities and Gregorian business dates, not HTTP/JSON
fields, system clocks or platform APIs. Constructor checks do not authorize
operations or replace server rules. Resource client mapping, form validation
and financial use cases remain separate tasks.

On 2026-11-07 all nine shared tests per target passed (zero failures/errors/skips),
along with `allTests` and all three shared artifact builds. This includes the
existing marker regression test. Native debug-cache warnings remain non-fatal.

## Exact JSON amount adapters

P10.05 adds serializers in `org.cuenora.shared.serialization`, without changing
or annotating the domain models. See [ADR-051](../docs/architecture/decisions/051-exact-json-amount-adapters.md).
Use explicit serializers for amount **fields**, not entire API resources:

```kotlin
val serializer = MoneyAmountSerializer(Currency.SEK)
val wire = Json.encodeToString(serializer, Money(12345, Currency.SEK)) // "123.45" (JSON string)
val money = Json.decodeFromString(serializer, wire)
```

`MoneyAmountSerializer(currency)` accepts the recorded input grammar and writes
canonical two-decimal strings. Currency comes from the resource; mismatches,
overflow and excess precision fail without rounding. JSON numbers are rejected.
`ProjectedAmountSerializer` maps arbitrary-precision projected minor units to
nonnegative two-decimal strings; use `.nullable` for an open tail.
`ReportAmountSerializer` validates signed two-decimal report strings without
narrowing or changing their text. None uses floating point or locale formatting.
Rates and rational accrual integer strings are not monetary amount fields.

On 2026-11-07 all 14 tests per Android-host/iOS-simulator target, `allTests`,
all three shared artifacts and independent snapshot verification passed. Existing
Xcode/debug-cache limitations still apply.

The JSON runtime is pinned at 1.11.0; handwritten serializers require no compiler
plugin. Full resource DTOs, HTTP integration and cross-language fixture tests
remain P10.06/P10.11. Never log financial payloads or raw JSON parser exceptions.

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
module adds kotlinx.serialization JSON 1.11.0 for amount adapters, but no HTTP client.
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
Both retain the `SharedModule.name` marker and expose the P10.04 domain models;
these models do not implement financial workflows.
On 2026-11-06, all three builds passed locally using JBR 25.0.3 and Xcode 27.2.
Inspection confirmed the marker in Android bytecode and both framework headers;
Apple archives identify iOS device/simulator separately with minimum iOS 15.0.
The [execution plan](../docs/plans/active/2026-11-06-kmp-foundation.md) records
observed commands, limitations and verification. This does not complete Phase 10
or prove runtime UI behavior. The [P10.02 execution record](../docs/plans/active/2026-11-06-native-hosts.md)
tracks host verification separately. CI and E2E setup remain P10.12 and Phase 13.
