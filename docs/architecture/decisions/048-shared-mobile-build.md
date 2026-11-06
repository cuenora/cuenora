# ADR-048: Independent shared mobile build

- **Status:** Accepted
- **Date:** 2026-11-06
- **Scope:** P10.01; extends ADR-001 without introducing application hosts

## Context and decision

Initialize `mobile/` as an independent Gradle build with `:shared`. Use Kotlin
2.4.21, Google's Android KMP library plugin 9.3.1 and Gradle wrapper 9.5.0.
Pin the wrapper distribution checksum; retain upstream wrapper scripts/JAR.
The [KMP matrix](https://kotlinlang.org/docs/multiplatform/multiplatform-compatibility-guide.html),
[AGP requirements](https://developer.android.com/build/releases/agp-9-3-0-release-notes)
and [Gradle JVM matrix](https://docs.gradle.org/current/userguide/compatibility.html)
inform this combination rather than selecting each latest version independently.

Compile common source to Android and static iOS ARM64 device/simulator frameworks.
Use the [dedicated Android KMP plugin](https://developer.android.com/kotlin/multiplatform/plugin),
not legacy `com.android.library` integration. Keep UI in future native hosts.
Use namespace `org.cuenora.shared` and framework name `CuenoraShared`.

Compile against installed Android SDK 36, provisionally requiring API 23 for this
library; final app support policy remains undecided. Emit JVM 17 bytecode while
running Gradle with installed JBR 25, avoiding a second JDK installation.
Intel iOS simulator support is not included in this ARM64 foundation.

## Consequences and alternatives

An independent build preserves backend commands and keeps mobile toolchains
optional for backend contributors. Separate native hosts and shared tests remain
P10.02–P10.03; the marker symbol only makes artifact contents inspectable.
Direct Gradle plugin pins are sufficient for one module; introduce a version
catalog when there is actual shared configuration to manage.

The wrapper adds vendored executable code and network downloads: verify its
upstream checksum and do not commit tool caches or SDK paths. Native compilation
requires macOS/Xcode. Xcode 27.2 installed locally exceeds Kotlin's published
26.4 matrix; preserve warnings and report local proof separately from official
compatibility. A side-by-side supported Xcode is an alternative if compilation
fails, at the cost of another large installation; never switch globally or
suppress compatibility checks automatically.
