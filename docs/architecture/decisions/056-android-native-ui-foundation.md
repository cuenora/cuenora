# ADR-056: Android-native UI foundation

- **Status:** Accepted
- **Date:** 2026-11-08
- **Related:** P11.01, ADR-049, DESIGN.md

## Context

The maintainer authorized both Pen design and Compose implementation of P11.01.
Cuenora needs a recognizable brand without forcing Android and iOS to share UI
or navigation conventions. The existing Android host is only a shared-module
integration marker; functional screens belong to later Phase 11 tasks.

## Decision

Use Android-only Material3 controls, with the existing Compose BOM, and Navigation
Compose 2.9.7 for a four-destination host. Keep the existing minimum/target SDK and
Kotlin/AGP versions. Navigation 2.10 raises the minimum to API 24 according to
[AndroidX release notes](https://developer.android.com/jetpack/androidx/releases/navigation#2.10.0-rc01);
2.9.7 preserves the current API 23 boundary. Its newer-version lint warning is
retained, not suppressed. Navigation owns the selected destination, saved state and
Back stack; no second route state or shared KMP presentation framework is added.

Map the documented blue semantic pairs to system-selected light/dark Material
schemes. Bundle licensed Bodoni Moda medium and IBM Plex Mono regular/medium;
there is no network font provider. Use scalable units and native system insets.
Wallpaper dynamic colors are deliberately not used because they replace the
approved brand palette.

Use a bottom bar only when full labels fit comfortably (440dp minimum at normal
font scale), a rail from 600dp, and a scrollable destination drawer for narrow
layouts or font scales above 1.2. The native drawer has explicit Back handling
and growing item heights. No iOS glass/capsule navigation is reproduced.
Navigation has no optional animation; native system behavior remains enabled.

## Consequences

Material3 provides reliable platform interaction semantics instead of a custom
control engine, at the cost of small platform-specific visual differences. The
fonts, financial vocabulary and colors retain Cuenora identity. iOS is not
modified and can later use its own SwiftUI structure.

New AndroidX runtime/test dependencies require app build, lint, instrumented
regressions and independent environment/dependency verification (Risk item 5).
Espresso 3.7.0 is explicit to avoid the older transitive version's removed
InputManager reflection on API 37. Instrumentation does not replace wider
accessibility/device coverage. Destination slots and component specimens are
explicitly unwired; no auth gates or financial behavior are implemented here.
