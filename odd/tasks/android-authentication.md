# Android authentication — P11.02
Objective: Native Android access through shared repositories, including the approved session lifecycle.
Branch: `feat/p11-02-android-auth`. Delivery: exception-ok, one focused P11.02 PR; forecast 1,200–1,800 authored lines including tests/docs; maintainer approved size exception on 2026-11-08. Runner: mobile Gradle shared tests and isolated Android instrumentation.

## Specs

- S1 — User request: "Comencemos la tarea P11.02 del plan maestro docs/plans/master-plan.md". Roadmap requirement: "P11.02 Implement login/registration through shared repositories." Deliver shared typed registration/login operations and Android native forms using the existing branded foundation. Validate inputs with existing shared validators, preserve generic registration acceptance, and expose safe actionable failures. Registration does not imply a session or email delivery. Verify HTTP contracts through MockEngine and Android interaction tests.
- S2 — User scope confirmation: "Sí, incluimos restauración de sesión, renovación de tokens y cierre de sesión". Restore via native SessionStore, coordinate rotating refresh credentials, and implement sign-out. Authentication remains distinct from verified-email financial authority. Verify restart, rotation/rejection, cancellation/uncertain outcome, secure-storage failure and logout behavior. No blind replay of refresh or financial mutations.
- S3 — Scope boundary: first-account onboarding remains P11.03. Existing financial placeholders must remain explicitly disconnected; no new financial operations, backend contract changes, iOS screens, verification/recovery journeys or remote deployment. Use isolated synthetic tests, not the configured hosted backend.

## Tasks

- T1 | S1, S3 | inline, shared authentication repository and wire tests | complete | `4b9e3ca` | Risk: item 2 (credentials).
- T2 | S2, S3 | inline, coordinated session lifecycle and deterministic tests | complete | `98d22da` | Risk: items 2 and 4 (secure storage, refresh/logout ordering).
- T3 | S1–S3 | inline, Android forms/session gate, platform checks and documentation | complete | `17a8eba` | Risk: item 2 (native credential/session integration).

## Log

- L1 — Original request: "Comencemos la tarea P11.02 del plan maestro docs/plans/master-plan.md".
- L2 — 2026-11-08 clarification: "Sí, incluimos restauración de sesión, renovación de tokens y cierre de sesión".
- L3 — Read-only mapping found existing ApiClient, auth validators, AndroidSessionStore and branded Compose foundation, but no authentication repository or session orchestration. Registration returns generic 202 without tokens. Resource RepositoryContext requires OwnerId, which login does not expose; this task will not invent an identity or decode opaque tokens.
- L4 — Implementation route: inline because repository, lifecycle and UI integrate one dependent behavior; no independent parallel writer units. A read-only explorer mapped the unfamiliar module. Independent verification is required for security and concurrency before claiming completion. RDD observed globally off; do not enable or start native review.
- L5 — Existing clean main was branched before writes. No source implementation or tests executed yet. All verification pending. Delivery forecast exceeds 400 lines; resolve chain strategy or an explicit focused size exception before the first work-unit commit. No push/PR/remote execution authorized.

- L6 — 2026-11-08 user decision: "Acepto una unica PR con excepción de tamaño". One focused PR with reviewable local commits; publication remains unrequested.

- L7 — T1/T2 focused RED observed as unresolved new symbols before implementation; GREEN observed on Android host for repository and lifecycle tests. UI RED observed for missing AuthenticationRoot; initial UI compile corrected the TextFieldBuffer API to use replace. Full suites, device tests and independent verification remain pending. ADR-057 records conservative clear-before-refresh and process-owned coordination, with interrupted renewal requiring fresh sign-in.

- L8 — Shared suites passed: 90 Android-host and 91 iOS-simulator tests; all shared artifacts built; five real Android Keystore tests passed. App lint error (unused BoxWithConstraints) fixed. An existing component assertion failed after authentication tests; isolated diagnostics showed visible error text with an open IME but a visibility-query false negative. Dismissing IME retained assertIsDisplayed and made the auth-to-component sequence pass. New deadline and Android error-state tests added; final full rerun pending.

- L9 — Final own checks passed: shared 91/92, allTests and three shared artifacts, app assemble/lint (four known warnings), app instrumentation 19/19. Prior Keystore 5/5 remains valid (storage source unchanged). Independent verification and final docs/repository readback remain pending; no completion checkbox changed yet.

- L10 — Independent Risk2/Risk4 verifier returned PASS for S1–S3, six executable scratch probes passed, baseline/candidate validator+financial-contract 18/18 had identical testcase outcomes and existing shared resource sources were byte-identical. No blocker required correction. Six independent probes retained as readable AuthSessionRecoveryTest regressions; final common-target rerun pending after adding those tests. Production implementation is unchanged from the independently verified snapshot.

- L11 — Expanded shared regression suite passed 97 Android-host/98 iOS-simulator, zero failures/errors/skips; allTests passed. Parent independently replayed the six scratch probes; production sources match independently verified bytes. No source-mutating Kotlin formatter is configured; diff whitespace checks are the proportional normalization check. Local work-unit commits and final cleanup remain before delivery.

- L12 — Work-unit commits: T1 `4b9e3ca` (236 authored lines), T2 `98d22da` (547), T3 `17a8eba` (426); total 1,209 before closing documentation. The session unit exceeds the advisory 400-line heuristic because its complete ordering/recovery tests belong with the transaction behavior; the single future PR size exception was explicitly approved. RDD remained globally off (disabled/unmanaged), no native assessment/review started. Parent scratch re-execution ran all 9 Gradle tasks including all six probes successfully. Source bytes remained unchanged through commits; no formatter/hook changes. Publication remains pending.

- L13 — Final readback: repository validator passed 636 Git-visible files; JUnit guards confirmed 97/98 shared tests, platform XML confirmed 19 app and 5 Keystore tests, independent scratch XML confirmed 6 probes. All dedicated emulators stopped and four owned external scratch roots removed after retaining executable regressions and durable evidence here. No human AVD or remote backend was touched. All T1–T3 outcomes complete; no failed/pending required checks remain. Limitations and baseline warnings are retained in the execution record. Next action, only if requested: publish the one approved-size-exception PR.

- L14 — 2026-11-08 subsequent maintainer authorization: "Sí, lo autorizo" to installing the existing debug APK on the connected physical Android phone and testing the scoped auth flows against https://cuenora.carf.dev with credentials entered only by the maintainer. Samsung SM-S908B/API36 installation succeeded. Maintainer reported generic registration acceptance, login to Home/Sign out, restoration after force-stop/relaunch, online logout and persistence after restart, offline logout with explicit remote-unconfirmed warning, and continued Welcome back after offline restart. These are user-observed UI outcomes, not captured network/server telemetry; restore exercises refresh by implementation, but timer-driven foreground renewal remains unverified. Detailed evidence/limits appended to the execution record; no backend administration, credential collection, financial data or code changes. Publication is still unrequested.
