# Minimal iOS host

P10.02 adds the native SwiftUI `Cuenora` app. Its only content is
`SharedModule.shared.name` imported from `CuenoraShared`; it does not duplicate
shared behavior, make network requests or implement the product design system.
The development bundle ID `org.cuenora.ios` and iOS 15 minimum are provisional,
not App Store reservations or a release support promise. Only ARM64 device and
simulator builds are configured.

## Build without signing

First configure Java and the Android SDK as described in the
[mobile setup](../README.md). From `mobile/`, use an external DerivedData directory:

```bash
scratch=$(mktemp -d)
xcodebuild -project iosApp/Cuenora.xcodeproj -scheme Cuenora \
  -configuration Debug -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$scratch/simulator" CODE_SIGNING_ALLOWED=NO build
xcodebuild -project iosApp/Cuenora.xcodeproj -scheme Cuenora \
  -configuration Debug -sdk iphoneos -destination 'generic/platform=iOS' \
  -derivedDataPath "$scratch/device" CODE_SIGNING_ALLOWED=NO build
```

For Xcode IDE builds, open `Cuenora.xcodeproj` with a valid Java environment;
an already-running Xcode may not inherit shell changes. No development team or
provisioning profile is checked in. Signed installation remains separate work.

## Shared framework integration

The pre-Sources build phase invokes `:shared:embedAndSignAppleFrameworkForXcode`
using Xcode's SDK/configuration. Following Kotlin's
[direct integration](https://kotlinlang.org/docs/multiplatform/multiplatform-direct-integration.html),
dependency analysis and **this target's** user-script sandbox are disabled so
Gradle can access the sibling shared build and tool caches. No global sandbox
setting changes. The static framework is linked, not copied as an app resource.

Both unsigned Debug builds passed locally on 2026-11-06 with Xcode 27.2 and JBR 25.
This does not establish official Kotlin compatibility or prove UI execution:
no simulator/device was launched, no signing performed and no backend contacted.
Shared tests were subsequently added in P10.03; later iOS work extends this
project, not a competing host.

## App-hosted Keychain tests

P10.07 adds `CuenoraSessionTests` to the existing scheme. These six Swift XCTest
cases call the shared public SessionStore API, including pair replacement/clear,
fresh instances, namespaces, concurrent writers, corruption, rejected input and
native device-only/non-sync attributes. They make no backend requests.

With the documented Java/SDK environment, from `mobile/`, create a disposable
simulator for the installed runtime and keep build/results outside the worktree:

```bash
scratch=$(mktemp -d)
device=$(xcrun simctl create CuenoraSessionTests \
  com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro \
  com.apple.CoreSimulator.SimRuntime.iOS-26-5)
trap 'xcrun simctl shutdown "$device" >/dev/null 2>&1 || true; xcrun simctl delete "$device"' EXIT
xcodebuild -project iosApp/Cuenora.xcodeproj -scheme Cuenora \
  -configuration Debug -destination "platform=iOS Simulator,id=$device" \
  -derivedDataPath "$scratch/DerivedData" -resultBundlePath "$scratch/KeychainTests.xcresult" \
  CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=- CODE_SIGNING_ALLOWED=YES \
  CODE_SIGNING_REQUIRED=NO DEVELOPMENT_TEAM= test
```

Adjust runtime/device identifiers to an installed compatible simulator. Signing
is local ad-hoc, with no Apple account, team or custom entitlement. Never run on a
personal device or change real Keychain items. Each test uses a unique synthetic
namespace and tears it down. The trap removes only the simulator created above;
retain the external result bundle until inspection, then remove the scratch.

The bundle loads shared Kotlin symbols from the app, not a second linked static
framework. Debug disables the app's debug dylib to provide this test host;
Release behavior is unchanged. The test target requires iOS17 for SDK XCTest;
the provisional app/shared minimum remains15.

On 2026-11-07 all six app-hosted cases passed on iOS26.5. The Native CLI runner
could not access Keychain; its six service cases were migrated here, not skipped.
Gradle `:shared:iosSimulatorArm64Test` runs 32 common tests plus the CFBoolean
bridge regression. Run both suites: `:shared:allTests` alone does not prove
Keychain behavior. Xcode/Kotlin compatibility and native debug-cache warnings
remain as documented in the mobile setup guide.
