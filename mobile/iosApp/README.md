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
Shared tests remain P10.03; later iOS work must extend this project, not create
a competing host.
