# Execution plan: Android income and expense forms

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.05
- **Tracking issue / PR:** Not created; one future size-exception PR approved
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Record dated income/expense with optional account through one reviewed form reached from Home, Transactions, and account details.

## Existing state and evidence
Shared transaction commands, HTTP adapter and validation exist. Android browsing and onboarding exist; financial forms and category adapter do not. See [task specification](../../../odd/tasks/android-income-expense.md).

## In scope
Form, review, optional category/description/account, exact money/date validation, authenticated posting, same-command/key uncertain-write recovery, three entry points and tests.

## Out of scope
Transaction history/filtering/editing, transfers, full dashboard, remote publication, hosted or physical-device tests.

## Constraints and invariants
Follow financial and identity policies, F02, no floating-point money; no account means no account balance effect. Ordinary income cannot target savings. Durable pending operation is not an offline draft. Existing auth/onboarding/browsing remain compatible.

## Steps
- [x] Shared transaction writer and encrypted journal with tests.
- [x] Read-only category catalog adapter with tests.
- [x] Native form, review, recovery and all entry points with tests.
- [x] Independent verification and completion evidence.

## Acceptance criteria
- [x] Task specification S1–S3 and F02 verified.
- [x] Relevant tests and docs updated.
- [x] Security, owner isolation and async ordering checked independently.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared Android host / iOS simulator tests | Pass | 136 / 137, zero failures/errors/skips | Passed |
| Android instrumentation/build/lint | Pass, baseline warnings disclosed | 42 API37 tests; assemble/lint pass, four baseline lint warnings | Passed |
| Repository checks | Pass | `python3 scripts/check_repository.py` passed; diff whitespace clean | Passed |
| Independent security/ordering probes | Pass | 26/27 first pass; B1 retirement failure reproduced, corrected and exact isolated recheck 1/1 pass | Passed after bounded correction |

## Risks, assumptions, and decisions
- Risk items 2 and 4: authenticated writes, durable retry and asynchronous UI ordering.
- One future PR with size exception approved; local work-unit commits only. RDD globally disabled.
- Same command/key retries resolve uncertain outcomes; never silently post again after sign-in.

## Completion notes
Shared AAR and both debug iOS frameworks also built. Final own checks ran with the existing Android Studio JBR/SDK from `mobile/`: `./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test`, separately `:androidApp:assembleDebug :androidApp:lintDebug`, `:shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64`, and `:androidApp:connectedDebugAndroidTest` on an external isolated clone of the existing API37 AVD. No production/backend calls or physical devices were used.

B1: a non-cooperative failed profile response republished Failed after retirement. The parent retained a deterministic RED regression, guarded failure publication centrally, observed GREEN and received the independent B1-only passing recheck. Independent owner-switch probe is retained. Existing 15 auth/financial-validation tests passed both baseline `0853190` and candidate. Final native suite was rerun after correction and passed 42/42.

Four baseline lint warnings remain: OldTargetApi, Navigation Compose update advisory, DataExtractionRules, MissingApplicationIcon. Existing iOS Keychain cast and non-fatal Native cache-link warnings remain. Initial worker test-first attempts were blocked by unset Java/concurrent missing symbols (not claimed as observed RED); the parent state-machine and bounded corrections have observed RED/GREEN. No hosted/physical-device/API23/full accessibility proof. S2 real application wiring is source-verified; native tests exercise shared form and entry callbacks rather than a hosted real-Activity financial journey.

P11.06 history/filtering and later dashboard work remain pending. One future size-exception PR approved, no remote publication performed. See ADR-060 and the task log for work-unit commits.
