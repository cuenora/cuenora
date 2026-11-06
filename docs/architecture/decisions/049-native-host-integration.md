# ADR-049: Minimal native hosts consume the shared library

- **Status:** Accepted
- **Date:** 2026-11-06
- **Scope:** P10.02; extends ADR-001/ADR-048

## Decision

Use a separate Android application module with native Compose and an Xcode
project with native SwiftUI. Both render `SharedModule.name` from the existing
library, not duplicated client logic. This is build integration, not a product
screen, navigation system or HTTP client. `https://cuenora.carf.dev` is the
maintainer-provided backend origin for future P10.06 work, not contacted here.

Use AGP's built-in Kotlin for Android, the Kotlin-matched Compose compiler and
pinned Compose BOM/Activity dependencies. Current AndroidX metadata requires
host compileSdk 37; retain provisional min23/target36 rather than suppress checks.
Development IDs `org.cuenora.android`/`org.cuenora.ios` and iOS 15 minimum are not
reserved store identifiers or finalized distribution support policy.

Use Kotlin's [direct Xcode integration](https://kotlinlang.org/docs/multiplatform/multiplatform-direct-integration.html):
a pre-Sources Gradle phase builds the framework matching Xcode's SDK/configuration.
Disable dependency analysis for that phase and user-script sandboxing **only for
this target**, because it must access the sibling shared build and tool caches.
No global sandbox/security configuration changes. Link the static framework;
do not embed it as a resource. Do not commit development-team or signing data.

## Tradeoffs and boundaries

Direct integration avoids CocoaPods/SPM or a project-generator dependency but
requires a valid Java environment for Xcode and a target-local unsandboxed build
script. A prebuilt framework would avoid that phase but require separate artifact
production/synchronization; it is unnecessary for this source monorepo baseline.
Android uses ordinary development debug signing, isolated during checks; release
signing remains separate. No API requests, transport exceptions or network
permissions are introduced. Native runtime, shared tests and UI behavior remain
unverified by compilation. Extend this same Xcode host for P12.01; do not create
a competing project or mark later iOS tasks complete automatically.
