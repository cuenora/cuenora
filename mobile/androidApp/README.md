# Android native foundation

P10.02 introduced the integration host. P11.01 adds native Compose navigation,
Cuenora light/dark typography and colors, and reusable base controls. Four
unwired destination slots are explicit about their scope. The component preview
uses synthetic examples only. No backend/authentication wiring or network
permission is added. See the [Android design handoff](../../docs/product/android-foundation/README.md).

## Build and check

From `mobile/`, using the JBR/SDK configuration in the [mobile setup](../README.md):

```bash
./gradlew --no-daemon :androidApp:assembleDebug :androidApp:lintDebug
```

The APK is under `androidApp/build/outputs/apk/debug/`. Android's default debug
signing makes this development APK installable; release signing is not configured.
For isolated checks, set `ANDROID_USER_HOME` to an external temporary directory
so the build creates a disposable debug keystore instead of using local credentials.

The development identifier `org.cuenora.android`, minimum SDK 23 and target SDK 36
are provisional, not reserved store identifiers or a final app support policy.
Compilation uses SDK 37 because the selected AndroidX libraries require it;
this does not change the target or minimum SDK.
Jetpack Compose libraries use BOM 2026.09.00; Activity Compose is pinned to 1.13.0.
The Compose compiler follows Kotlin 2.4.21 independently of the BOM.

## Instrumented foundation tests

Start an isolated Android emulator or attach an authorized test device. Run:

```bash
./gradlew --no-daemon :androidApp:connectedDebugAndroidTest
```

Tests cover destination selection/restoration/Back, responsive navigation,
light/dark brand pairs, invalid input, unavailable/busy controls and safe dialog
focus. These are app UI tests, not shared-library or financial end-to-end tests.
JUnit reports are under `build/outputs/androidTest-results/connected/debug/`.
If the default 512MiB Gradle heap thrashes while dexing instrumentation, pass
`'-Dorg.gradle.jvmargs=-Xmx3g -XX:MaxMetaspaceSize=1g'` locally; no global Gradle
configuration change is needed. Navigation 2.9.7 retains API 23; newer 2.10 requires API 24. The corresponding
newer-version lint warning remains visible. Espresso 3.7.0 is explicit because the older
transitive version calls a removed InputManager method on the API 37 emulator.

## Earlier integration evidence


On 2026-11-06, `assembleDebug` and `lintDebug` passed with JBR 25.0.3. The first
SDK 36 build failed AndroidX AAR metadata validation; changing only `compileSdk`
to 37 resolved it without suppression. APK inspection confirmed the label,
development ID, SDK bounds and a call to the shared marker getter.

Lint reports no errors and three unsuppressed scaffold warnings: old target API,
missing data-extraction rules and missing application icon. Release policy,
backup behavior and branded assets are not established by this integration host.
That earlier integration check did not establish UI runtime behavior. Current
foundation verification is recorded in the linked P11.01 execution record.
