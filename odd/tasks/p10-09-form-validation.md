# P10.09 — Shared form validation
Implement pure common Kotlin form validation. Branch: `feat/p10-09-form-validation`.
Delivery: exception-ok, one future PR; forecast 700–1000 authored lines; no publication authorized. Runner: mobile Gradle Android-host/iOS simulator tests.

## Specs

- S1: "Comencemos la tarea P10.09 del plan maestro docs/plans/master-plan.md" — "Implement shared form validation." Cover account and transaction form input using existing domain rules, exact amounts, Gregorian dates, accountless entries and transfer cross-field checks. Pure typed validation results with field identifiers, no raw input or credentials in errors. Reuse existing domain constructors and exact amount serializer. No native UI or backend changes.
- S2: "incluimos también autenticación, recurrencias y condiciones de ahorro" — Include registration/login/recovery/reset/verification input validation, recurrence terms and savings terms/rates. Follow implemented backend syntax and deterministic constraints; no authentication orchestration, schedule generation or interest calculation. Financial edit forms validate the complete merged form; PATCH construction remains repository/caller responsibility.
- S3: "Seguimos con ese enfoque" — Accepted approach: "Recomiendo validar localmente campos y reglas deterministas, dejando esas comprobaciones al servidor. Evita nuevas dependencias, aunque algunos errores aparecerán recién al enviar." IDNA, common-password checks and supported-zone catalog remain server-side. Local success explicitly means locally acceptable, never server-approved. Do not reject international domains merely because IDNA is unavailable. Preserve passwords exactly; login must not enforce signup rules.
- S4: Verify each validator through deterministic commonTest RED→GREEN on Android host, then Android/iOS suite and all three shared artifacts. Document limits and no UI/server integration claim. Existing shared behavior remains unchanged.

## Tasks

- T1 | S1–S4 | delegated (parallel independent auth/financial units, disjoint surfaces) | completed | auth validation, common results and tests | Risk: item 2 (credential form policy); commit `451a27d`.
- T2 | S1–S4 | delegated (parallel independent auth/financial units, disjoint surfaces) | completed | financial validation and tests | Risk: none (new opt-in form API); commit `ed78a99`.
- T3 | S1–S4 | inline | completed | integrate docs and own checks, independent security verification | closing commit `docs(shared): close P10.09 validation task` (resolve with git log -- odd/tasks/p10-09-form-validation.md).

## Log

- L1 (2026-11-07): "Comencemos la tarea P10.09 del plan maestro docs/plans/master-plan.md"
- L2 (2026-11-07): "incluimos también autenticación, recurrencias y condiciones de ahorro"
- L3 (2026-11-07): "Seguimos con ese enfoque"
- Existing main is clean; native review is disabled globally. Mapping found reusable AccountName, BusinessDate, MoneyAmountSerializer and Transaction invariants. No common form API currently exists. Source changes must not alter existing domain/repository contracts.
- Parallel surfaces: auth owns Validation.kt, AuthValidation.kt and AuthValidationTest.kt; financial owns FinancialValidation.kt and FinancialValidationTest.kt. Auth provides ValidationIssue(field: String, code: String) and FormValidation(issues: List<ValidationIssue>) with isLocallyValid, serverChecksRequired always true; no raw inputs in results. Financial may use those declared types without editing their files. String field/code values are stable technical identifiers, not UI copy.

- Own checks: Auth initial RED unresolved symbols; separator regression RED assertion observed then GREEN. Financial stub RED failed 7 tests then GREEN. Combined suite/artifacts passed: 73 Android-host and 74 iOS-simulator tests, zero failures/errors/skips; allTests, AAR and two frameworks passed. Repository and diff checks passed. Existing Native cache-object and Keychain cast warnings remain. Independent verification pending.
- Delegated diff assessment: high due to explicit risk item 2. Initial assessment unavailable because new files were untracked; after staging exact intended surfaces assessment returned high. Native review remains disabled; no lifecycle started.

- Independent verification passed with no in-scope blockers: baseline 58 tests, candidate 73 plus 2 probes; prior testcase identities/stdout/stderr unchanged. Interest on normal accounts is supported; quote numeric overflow remains server-side. Scratch /tmp/cuenora-verify-forms.CYejJa retained because cleanup command was denied by runtime.
- ADR-055 documents advisory semantics. Maintainer approved one future PR with size exception; local work-unit commits may proceed. No publication authorized.

- L4 (2026-11-07): "Autorizo una única PR con excepción de tamaño". Use one future PR, separate auth and financial work-unit commits. Entire feature is the PR boundary; no chain. Current authored forecast approximately 920 lines.

- Work-unit evidence: `451a27d` authentication/results/tests/ADR, 304 authored lines, risk item 2 independently verified; `ed78a99` financial validation/tests/usage docs, 529 authored lines, risk none. Native review disabled/unmanaged for both, so no review lifecycle invoked. Tests and artifact proof precede commits; no source changes afterward. Single future PR holds both units plus closing docs; size exception approved.
- Verification command from mobile with documented JBR/SDK environment: `./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test :shared:allTests :shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64` passed. `python3 scripts/check_repository.py`, `git diff --check` and `git diff --cached --check` passed. No UI, hosted API or production proof claimed.
- Closing local P10.09; master plan checked only after own and independent evidence. No push, PR or merge authorized/performed. Temporary independent-verifier scratch retained after runtime denied removal.
