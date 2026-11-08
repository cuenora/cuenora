# Android account list and details — P11.04
Objective: Connect Android Accounts to paginated server records and exact posted balances.
Branch: `feat/p11-04-account-list-details` (base `b47a496`); delivery: exception-ok (single future PR, not published); runner: Gradle shared tests and isolated Android instrumentation.

## Specs

- S1: "Comencemos la tarea P11.04 del plan maestro docs/plans/master-plan.md" — "P11.04 Implement account list/details." Implement the Android Accounts destination through the existing authenticated shared repository, including paginated listing, account selection, and metadata/current posted balance details. Include loading, empty, failed-read and retry states without fabricated balances or mixed-currency totals.
- S2: "Sí" to "¿Dejamos los movimientos para P11.06?" Detail is limited to name, type, currency, status and posted balance. No activity, pending payments, account management or additional creation workflow in this task. Preserve first-account onboarding and existing navigation/session behavior.

## Tasks

- T1 — S1–S2 — inline — verified: add an authenticated read adapter and tests using the existing session ordering and server profile identity; no new HTTP contract. Risk: items 2 and 4. Commit: `3558d43`.
- T2 — S1–S2 — inline — verified: Android-only list/detail state and Compose integration with pagination, retry, Back and exact amounts; regression tests and documentation. Risk: items 2 and 4 (sensitive session data and async reads). Commit: `22c7918`.
- T3 — S1–S2 — delegated verification — verified: independent security/order probes after own checks; full-feature verification and delivery record. Risk: items 2 and 4. Proof commits: `3558d43`, `22c7918`; passive closure recorded below.

## Log

### L1 — 2026-11-08 — Original request

Comencemos la tarea P11.04 del plan maestro docs/plans/master-plan.md

### L2 — 2026-11-08 — Scope confirmation

Parent asked: "P11.04 define listado y detalle de cuentas, pero el flujo de producto también menciona movimientos dentro del detalle; P11.06 aborda su listado y filtros. Recomiendo limitar P11.04 a cuentas paginadas, detalle con nombre, tipo, moneda, estado y saldo contabilizado, más carga, errores y reintentos. ¿Dejamos los movimientos para P11.06?"

User: "Sí"

### L3 — Existing evidence and route

Clean main at b47a496 includes P11.03. Accounts is a placeholder. AccountRepository.list already supplies exact posted balance and cursor; there is no single-account GET contract. Details display the selected server list record and explicitly describe its read-time meaning. Reuse AuthSession.authorized, HttpOnboardingRepository.profile and HttpAccountRepository.list, never a locally guessed owner or retained bearer. Android owns native presentation; P10.10 remains deferred. No backend, schema, dependencies or iOS UI changes planned.

A bounded read-only explorer mapped this flow. Implementation stays inline: one dependent behavior, no independent parallel writer units. Independent verification is mandatory for security/session isolation and async ordering. All mutable test scratch stays outside the repository; no physical device, remote backend, publication or push authorized.

Forecast: approximately 800–1100 authored changed lines including tests and execution records. Delivery choice is pending before the first work-unit commit; no inherited size exception from a previous task. RDD is disabled by the global user setting; no native review will be started.

### L4 — 2026-11-08 — Delivery approval

Parent asked: "¿Autorizás una única PR con excepción de tamaño y commits separados por comportamiento, sin publicarla todavía?"

User: "Sí"

Proceed with one future PR and separate coherent local commits. No publication authorized.

### L5 — Implementation and initial checks

Observed RED for the missing AuthenticatedAccountReader, AccountBrowser, AccountsScreen and accountsContent APIs. The reader's four focused tests passed, then shared suites passed 113 Android-host / 114 iOS-simulator tests with no failures/errors/skips. App assembleDebug/assembleDebugAndroidTest and lintDebug passed; four baseline lint warnings and Kotlin/Native cache warnings remain.

The first instrumentation attempt ran zero tests because a background emulator outlived its launching shell only briefly. Keeping the emulator and Gradle in one supervised shell fixed device detection without source changes. Eight focused native tests then ran: six passed; two UI assertions failed (notice includes the existing Error prefix; offline toggle preceded observed first-page readiness). Correct the assertions/synchronization without weakening financial or recovery checks, then recheck and run the full suite. No full native suite has executed yet.

### L6 — Final verification and baseline test repair

Final shared suites passed 115 Android-host / 116 iOS-simulator tests, zero failures/errors/skips; allTests, Android AAR and both iOS frameworks passed. Final focused native tests passed 16/16 and full API37 native suite passed 35/35. Parent independently replayed the reader 6 probes and app assembleDebug/lintDebug; both passed. Repository/JUnit guards and whitespace checks passed. No source-mutating Kotlin formatter is configured.

Independent verifier returned PASS S1/S2, no candidate-caused blockers. Probes covered renewed bearer, read/logout ordering, denied owner change without credential mutation, retired browser/non-cooperative late results, cursor cycles/mixed owners and exact amount bounds. All 43 production source/resource hashes matched the independently verified candidate after test-only updates. Two shared and two Android independent probes were retained as readable regressions. A scoped recheck caught lost clear-event tracing during test-helper extraction; the original six-event trace was restored, inspected and passed on both targets.

The first full native run had four auth failures. Independent b47a496 baseline and candidate each reproduced identical 6-test/4-failure outcomes: submit taps below an IME-clipped viewport made zero repository calls. The test-only submit helper now dismisses the keyboard and scrolls to the button; all original assertions are byte-preserved after reversing helper substitutions. Independent baseline with the identical helper passed all 6; final candidate full 35 passed. No auth runtime behavior was changed.

Four baseline lint warnings remain: old target, Navigation version, extraction rules and launcher icon. Existing Kotlin/Native cache warnings and Keychain cast warnings remain visible. Backend/schema/dependencies/auth source/iOS UI are unchanged. No live backend, physical-device/API23 or complete assistive-technology verification is claimed. RDD stays disabled/unmanaged; no native review was started. Local behavior commits and scratch cleanup are the remaining delivery steps, not publication.

### L7 — Local work-unit delivery

T1 committed as `3558d43` (`feat(shared): add owner-bound authorized account reads`), 253 authored additions with tests/docs. Focused reader 6, final full 115/116 shared suites and three shared artifacts passed. Runtime boundary: synthetic MockEngine profile/accounts only, no hosted API. Rollback boundary: the new read adapter, its tests and shared account-repository guide section, leaving existing repositories/auth/runtime untouched. RDD disabled/unmanaged; no review invoked.

T2 includes Android presentation/integration plus its native tests, retained browser probes, baseline test-tap repair and ADR059. T3 closes independent evidence and delivery documentation. The single-PR size exception applies; no publication authorized. All functional checks are complete; only commits/bookkeeping remain.

### L8 — Final closure

T2 committed as `22c7918` (`feat(android): implement account list and balance details`), 546 insertions+16 deletions including native tests, baseline-tap repair, docs and ADR059. This coherent native integration exceeds the advisory 400-line slice heuristic under the explicitly approved single-PR exception. Rollback boundary: Accounts presentation/reader factory/optional shell integration and its tests/docs, preserving existing onboarding/auth implementations. T3 proof is retained with the behavior in T1/T2, not split into test-only behavior commits. A final passive documentation commit records the verified roadmap state and this logbook.

Final readback confirmed shared 115/116 and native 35 XML tests with zero failures/errors/skips; JUnit report guards, repository validator and whitespace checks passed. All three exact owned external scratch roots were removed after durable evidence and executable regressions were retained. Disposable emulators were stopped and adb inventory is empty. No human AVD, personal data, remote backend, issue/PR, push, merge or deploy was used.

S1/S2 and T1/T2/T3 are complete locally. P11.04 is checked with explicit verification limits. Activity remains P11.06. Next action only upon explicit authorization: publish the one approved-size-exception PR using an authorized destination and session. RDD disabled/unmanaged throughout.

### L9 — Authorized physical-device verification

User: "Verificamos en el dispositivo real?"

Parent asked permission for installing the APK with preserved data and read-only
list/detail/navigation/refresh testing against https://cuenora.carf.dev, using the
existing session or credentials entered by the maintainer. User: "Sí".

The update failed INSTALL_FAILED_UPDATE_INCOMPATIBLE. The maintainer then answered:
"Si, y si no autorizo eliminar la actual para instalar la nueva aunque se pierdan datos"
to the request to compare the local debug key and rebuild if matching. Public-certificate
comparison found no match. Explicitly authorized uninstall, fresh install and MainActivity
launch succeeded on Samsung SM-S908B/API36. Local app data/session/journal were removed;
no server financial record was changed by those device commands. No credentials,
financial records, screenshots or hierarchy were captured; certificate scratch removed.

User independently answered "Sí" to each of six sequential checks: account list metadata
and balances; detail/list consistency; Android Back preserves list/context/position;
online Refresh accounts finishes without errors/duplicates; offline refresh presents
an error/Try again without fabricated empty/zero data; reconnect/Try again restores
correct records. These are maintainer-reported UI observations, not captured network
or server telemetry. Scoped manual read-only physical/hosted smoke passed. Pagination
beyond visible records, API23, timed renewal, financial writes and full accessibility
remain unproven on device. Initial pre-device scope limits in L3/L6/L8 are historical;
this later authorization expands only this bounded runtime verification. Publication
remains unauthorized. No runtime sources changed for this verification.
