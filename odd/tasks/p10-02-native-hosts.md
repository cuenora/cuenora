# P10.02 — Minimal native hosts
Chain: P10.01 `8d45d93` → `feat/p10-02-native-hosts` → `feat/p10-02-ios-host` → `docs/p10-02-native-hosts`. Delivery: auto-chain, feature-branch-chain (maintainer selected); forecast ~550 authored lines, no publishing authorized.
Runner: Gradle APK/lint, unsigned Xcode simulator/device builds, artifact inspection and repository check. Risk: items 2/5 (target-local Xcode script sandbox setting and new build dependencies).

## Specs

- S1: "Pasemos a la siguiente tarea P10.02." Roadmap: "P10.02 Create minimal Android and iOS hosts that consume shared logic." Both native hosts display the existing `SharedModule.name`; do not add financial behavior, navigation, auth or commonTest (P10.03).
- S2: "Recuerda que la url del backend es https://cuenora.carf.dev". Record the user-provided base URL; no network requests or HTTP client in this host task (P10.06).
- S3: "una cadena sobre la rama de funcionalidad". Keep Android and iOS reviewable as separate work units on the feature chain; do not push, create PRs or merge.

## Tasks

- T1 | S1/S3 | delegated; independent Android/iOS units launched together, disjoint surfaces | complete | `1341293`, 121 lines; APK/lint and independent probes pass; native review disabled/unmanaged.
- T2 | S1/S3 | delegated; parallel native host unit | complete | `90f1753`, 331 lines; both unsigned Debug builds and independent probes pass; native review disabled/unmanaged.
- T3 | S1–S3 | inline | complete | Integration documentation on `docs/p10-02-native-hosts`; independent S1/S2 pass, S3 branch boundaries recorded; current record commit.

## Log

- L1 (2026-11-06): "Pasemos a la siguiente tarea P10.02. Recuerda que la url del backend es https://cuenora.carf.dev"
- L2: Maintainer selected "una cadena sobre la rama de funcionalidad". Local chain only; planned slice boundaries: Android host, then iOS host, then passive verification record. P10.01 remains unmerged dependency. No PRs created.
- L3: Existing shared marker/framework names must be reused without duplication. Provisional development IDs `org.cuenora.android`/`org.cuenora.ios`; no store reservation or release support promise. Match library Android minimum 23 and iOS framework minimum 15.0; final distribution policy deferred. Android uses built-in Kotlin with AGP 9.3.1 and Compose compiler 2.4.21; verify current AndroidX SDK metadata rather than suppress it.
- L4: No existing native host test runner/business rule supplies meaningful RED; compile/import/link plus APK/framework inspection are the proportional integration checks. No fabricated no-op unit tests. Existing shared artifact builds provide regression coverage; independent verification required for items 2/5. Runtime UI, signing and backend connectivity are not established by compilation.
- L5: Android writer stayed within `mobile/androidApp/**` (118 lines); first build rejected compileSdk 36, one correction to host-only 37 passed APK/lint. min23/target36 retained. APK calls shared getter; debug signature verifies using isolated disposable key. Lint 0 errors, 3 documented warnings. iOS writer stayed within `mobile/iosApp/**` (331 lines); both unsigned SDK builds passed, shared symbols and platform IDs confirmed. Only target-local script sandbox disabled. Release signing remains excluded; standard Android debug signing is retained.
- L6: Independent fresh-copy verification found no implementation blockers. Candidate Gradle seam72 tasks, baseline shared22 tasks and both unsigned Xcode builds passed. Android getter invocation, no INTERNET permission, temporary debug signature and Apple platform IDs verified. Shared AAR contents/header APIs and backend bytes unchanged. Repository checks544/532 passed. No runtime launch; scratch retained outside workspace. Reviewable slices: Android host/root wiring121 lines, iOS host331 lines, then passive integration records.
- L7: Android commit `1341293` and iOS commit `90f1753` retained on separate local branch tips; integration records follow as passive child. S3 complete without publishing. Native assessment high (items2/5), but final preflight refused before mutation because global review mode is OFF; read-only mode/status confirmed `rdd_disabled`. Do not start/enable reviews or report approval. P10.02 marked complete locally after own/independent checks; P10.03/P12.01 remain pending. Keep execution plan active until merge.
