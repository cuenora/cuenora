# Minimal Android host

P10.02 provides a native Compose activity displaying `SharedModule.name` from
`:shared`. This is a build integration marker, not a product screen or theme.
There is no backend client, authentication, navigation or network permission.

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

## Verification boundary

On 2026-11-06, `assembleDebug` and `lintDebug` passed with JBR 25.0.3. The first
SDK 36 build failed AndroidX AAR metadata validation; changing only `compileSdk`
to 37 resolved it without suppression. APK inspection confirmed the label,
development ID, SDK bounds and a call to the shared marker getter.

Lint reports no errors and three unsuppressed scaffold warnings: old target API,
missing data-extraction rules and missing application icon. Release policy,
backup behavior and branded assets are not established by this integration host.
No emulator/device launch or UI runtime result is claimed. Shared tests remain
P10.03 and the application UI system remains P11.01.
