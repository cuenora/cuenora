# P11.05 — Android income and expense recording
Implement dated, optional-account recording from all F02 entry points.
Branch: `feat/p11-05-income-expense-forms`. Delivery: exception-ok, one future PR; forecast 1,800–2,800 authored lines. Runner: mobile Gradle shared/native tests.

## Specs
- S1: "Comencemos con la tarea P11.05 del plan maestro docs/plans/master-plan.md" — "P11.05 Implement dated income/expense forms with optional account." Follow approved F02 and existing money/date/account policies; review before save, optional category/description, no transaction history/editing/transfers.
- S2: "Entonces lo hacemos así" accepts Home, Transactions, and account-detail entry points sharing the form, with detail account preselected. Return to origin; no dashboard/history scope expansion.
- S3: "autorizo una unica PR con excepcion de tamano para esta implementación" — one future size-exception PR; no push/PR creation authorization assumed.
- Acceptance: income/expense, exact positive supported-currency amount, explicit valid past/today business date in confirmed profile zone, optional eligible account, applicable catalog category, review/cancel/success, authenticated owner checks, durable original-command/key recovery and no duplicate/automatic resubmission. Existing session, onboarding and browsing remain compatible.
- Checks: focused RED then GREEN shared tests; Android UI tests; shared Android-host and iOS simulator suites; Android build/lint/instrumentation; repository check; independent security/async verification. No remote or physical-device operations authorized.

## Tasks
- T1 [S1] delegated (parallel independent shared write unit): transaction backend and durable owner/origin-bound journal plus tests; complete, focused/shared/native journal checks pass; commit `7410896`. Risk: items 2,4.
- T2 [S1] delegated (parallel independent catalog read unit): authenticated category adapter plus tests; complete, focused/full shared checks pass; commit `10c94fd`. Risk: none (additive adapter).
- T3 [S1,S2] inline: Android state/form/review/recovery and three entry points, tests/docs; complete, final 136/137 shared and 42 native tests pass; commit `bbbfce2`. Risk: items 2,4.
- T4 [S1-S3] delegated verification: full feature independent probes after own checks, finalize execution record; complete, B1 corrected and isolated recheck passed; source commit `bbbfce2`.

## Log
- L1: User: "Comencemos con la tarea P11.05 del plan maestro docs/plans/master-plan.md"
- L2: User: "Pero hay una tarea más adelante que los incluya?" Verified no later explicit owner for Home/account-detail shortcuts; F02 already includes all three.
- L3: User: "Entonces lo hacemos así, autorizo una unica PR con excepcion de tamano para esta implementación"
- L4: Baseline 0853190, clean main. CodeGraph sync/exploration and bounded explorer mapped reusable shared transaction commands/validator, account reader and account journal. No transaction journal or category adapter. RDD off, global decision; do not start native review. Workers own disjoint new shared files; parent owns UI/integration and git. Artifacts English.

- L5: Own checks: focused recording/category host tests pass; RecordingSession missing-symbol RED observed before implementation. Writer tests authored first but initial RED was blocked by Java/concurrent compilation, not an observed test failure; bounded replay correction had observed RED/GREEN. Full shared suites pass (134 Android host/135 iOS simulator); Android build/lint pass; 42 isolated API37 instrumentation tests pass. Existing Native/iOS cast warnings remain; coroutine test opt-in normalized after run. No hosted or physical-device tests.
- L6: Replay must bypass current account eligibility and reach server original-key resolution even after archival. Added accountless zone confirmation using existing profile API. Independent verification next, Risk items 2 and 4.

- L7: Independent verifier first ran 27 tests: 26 passed, one new retirement-failure blocker. Parent retained equivalent regression, observed RED, guarded fail() against retired state, observed GREEN; independent exact B1-only recheck 1/1 passed. Retained owner-switch probe. Existing 15 auth/financial validation tests matched baseline.
- L8: Final post-correction own suites: 136 Android-host/137 iOS-simulator, zero failures/errors/skips; Android build/lint, shared AAR and two iOS frameworks pass. Final corrected native suite 42/42 passes on isolated API37 AVD. Four baseline lint and existing iOS/Native warnings disclosed in execution record. No source changes after those checks; docs/commits only. Parent repository/whitespace checks pass.
- L9: Commits: T2 `10c94fd` (153 additions); T1 `7410896` (532 additions, cohesive authenticated journal/replay behavior); T3 `bbbfce2` (816 additions/9 deletions, one form/state-machine/native integration with tests/docs). Larger cohesive slices use approved exception rather than code-golf. Final task-log closure is documentation only. All remain local; no push/PR. RDD disabled/unmanaged; staged native assess reported high due to explicit item 2, independent security/async verification completed.
- L10: No hosted/backend integration, physical-device/API23 runtime or full accessibility certification. Native component/form entry tests pass and real-Activity wiring is source-verified, not a hosted end-to-end journey. P11.06 history and later dashboard remain separate. Feature complete locally; one future size-exception PR ready for publication authorization.
