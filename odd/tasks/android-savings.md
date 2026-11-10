# P11.09 Android savings
Objective: integrate savings terms, forecasts and actual interest receipts. Branch: `feat/p11-09-android-savings`.
Delivery: exception-ok; maintainer approved one future size-exception PR on 2026-11-10; forecast 1,500–2,500 authored changed lines. Runner: mobile Gradle wrapper; `:shared:testAndroidHostTest`, `:shared:iosSimulatorArm64Test`, Android build/standalone lint and explicit isolated-emulator instrumentation.

## Specs
- S1: "Comencemos la tarea P11.09 del plan maestro docs/plans/master-plan.md" — roadmap wording: "Implement savings terms, forecasts, and actual interest receipts." Implement Android/shared integration over the existing savings and transaction contracts; no backend/schema changes absent a demonstrated gap.
- S2: Maintainer answered "Sí" to "Para la proyección, propongo que el usuario elija explícitamente las fechas de inicio y fin, sin un período preseleccionado que pueda confundirse con el próximo pago bancario." Require explicit dates; disclose snapshot and prior-accrual limitations.
- S3: Existing project invariants apply: exact monetary strings, explicit actual-interest confirmation, owner isolation and original-command recovery without automatic replay. Reuse existing transaction writer and immutable terms/rate streams.

## Tasks
- T1 | S1,S3 | inline | verified | shared savings reads/configuration and deterministic adapter tests; commit `6c31332` (294 authored lines incl. tracking).
- T2 | S1,S3 | inline | verified | authenticated lifecycle and secure recovery with retirement/order tests; commit pending.
- T3 | S1,S2,S3 | inline | pending | native savings workflows and tests/docs; commit pending.
- T4 | S1-S3 | inline plus delegated risk verification | pending | full applicable checks, independent items2/4 verification, closure; commit pending.

## Log
- L1 (2026-11-10): "Comencemos la tarea P11.09 del plan maestro docs/plans/master-plan.md"
- L2 (2026-11-10): "Sí" — explicit forecast-date selection approved; no default range.
- L3: Read-only explorer mapped existing backend terms/rates/forecast and shared InterestIncome command. Parent spot-checked savings forecast and configuration contracts. No implementation written. Clean main before feature branch creation.
- L4: Route inline: units have sequential dependencies, so no parallel-writer reason. Mapping delegated because cross-layer understanding exceeded bounded evidence. Security/session ordering invokes independent verification after own checks. No remote operations or physical-device access authorized.
- L5: Forecast exceeds 400 authored lines. Delivery decision required before first work-unit commit. No PR/push authorization implied.
- L6 (2026-11-10): "Una única PR con excepción de tamaño" — delivery exception approved; publication remains separately authorized.
- L7: Baseline `8920fcff15e824f4caa580bc69e1fb3ff7b842e0`. Shared repository/session RED observed for missing types; explicit configuration-conflict regression observed failing before correction. Focused savings tests and native-test compilation passed. No backend, schema, dependency or existing journal format changed.
- L8: Actual-interest receiving-account selection includes active normal and savings accounts; only the selected receiver is posted through existing TransactionRepository. New encrypted savings journal has a distinct namespace and stores natural-slot configuration or original keyed receipt. Definitive immutable configuration conflicts permit correction; uncertain receipt intents are retained.
- L9: Full own checks run as delegated per-action verification because this is large-task long build/suite work, with no source edits permitted. Local disposable emulator only, no connected-device discovery, physical devices or remote API. RDD read-only status: globally disabled; no native review will start.
- L10: Own proof: shared Android-host194 and iOS-simulator195 tests passed with zero failures/errors/skips. Shared AAR/device+simulator frameworks, APKs and standalone Android lint passed (zero lint issues). Disposable API37 full app109/shared-device7 tests passed, including savings UI5 and savings journal1. Emulator stopped and its process/ports confirmed absent. No source/config/resource changes across the153-entry proof manifest.
- L11: Independent security/order verification passed S1-S3: candidate77 cases including11 scratch probes; baseline53 unchanged legacy cases matched with no missing/changed outcomes. Probes covered natural-slot conflicts, arbitrary-precision forecast, original receipt body/key, owner changes, retirement after profile/storage suspension, completed logout, storage failure and explicit recovery. Parent reran focused savings host tests successfully; repository validator and diff check passed.
- L12: No backend/schema/dependency or existing journal-format changes. No hosted or physical-device smoke, minimum-supported-API or full accessibility proof claimed. Kotlin/Native cached-archive warnings remain visible; own test command succeeded before its shell wrapper hit zsh's reserved status variable. First external-AVD launch failed before boot without ANDROID_AVD_HOME; corrected environment booted owned disposable AVD, not an existing device. RDD remains disabled/unmanaged.
- L13: T1 committed as `6c31332` (294 authored lines). RDD disabled/unmanaged. Rollback boundary: new shared savings repository/tests and passive feature records; no prior adapter behavior changed. T2 is one coherent authenticated write/recovery behavior; keeping its storage, lifecycle and tests together exceeds the advisory400-line slice target under the approved exception.
