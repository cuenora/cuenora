# Android additional account creation
Objective: Offer normal account creation from Accounts after first-account onboarding.
Branch: `feat/p11-06-transaction-history`, baseline `66f343e`; delivery: exception-ok (one future PR, behavior commits), publication authorized within the transaction PR. Runner: shared Android/iOS tests, native isolated API37 tests and app build/lint. Pending splash edits are unrelated and must remain untouched/uncommitted.

## Specs
- S1: User: “No encuentro como crear una segunda cuenta. O sea, el ombording me deja crear la primera, pero luego de que ya tengo una, como añado una segunda?” Then “Sí” to implementing “Add account” in Accounts, reusing secure creation without depending on first-account onboarding.
- S2: Reuse existing Normal/Savings, currency, account-name and optional exact dated opening validation, review, verified identity/profile-date gates and original-key recovery. Preserve first-account onboarding, existing accounts, and one owner/origin-bound pending journal; no automatic writes or new backend contract. These are existing repository invariants, not new product fields.
- S3: Return to refreshed Accounts after success; cancellation/Back uses existing discard semantics and safe top/bottom/IME insets. Interrupted/unknown writes retain original command/key; signing out retires the coordinator and cannot permit late publication or a later POST.

- S4: User: “Sí, tal vez es del cambio que hicimos para que el appbar tuviera el mismo color que la barra de estado”. Authorized a bounded follow-up on the failed status-bar test; verify the hypothesis without assuming cause or weakening the color assertion.

## Tasks
- T1 — S1/S2/S3 — inline — complete — shared additional-creation intent and retirement with deterministic common tests; commit `78da619`.
- T2 — S1/S2/S3 — inline — complete — Accounts entry and shared form host with native regressions; commit `cc16d77`.
- T3 — S1/S2/S3 — delegated checking (risk items 2/4: identity, journal and async ordering) — complete — own suites, independent verifier, docs and closure.

- T4 — S4 — inline — complete — same-frame test oracle and regressions; commit `87ffc31`.

## Log
- L1: Original issue and authorization are quoted in S1. User authorized implementation only, not publication or financial smoke writes on development backend.
- L2: Existing onboarding begin accepts Empty only; Accounts is read-only. Backend account creation inserts new accounts via existing authenticated POST and preserves permanent operation-key replay. No new endpoint required. Single coordinator/journal avoids racing independent account writers. First onboarding remains the default intent; explicit additional intent refreshes verified profile and checks pending recovery before a new form.
- L3: Route inline: one dependent behavior, no parallel-writer reason. Forecast450–650 authored lines including tests/docs; delivery/chain decision required before committing if over400. Existing splash changes preserved exactly. No new source commits until delivery choice resolved. New records/openings are not high-risk item1 by themselves; items2/4 require independent verification.

- L4: T1 observed common compile RED (beginAdditional missing), then8 focused additional/original common tests GREEN. T2 native compile RED (AdditionalAccountHost missing) then compile GREEN; runtime checks pending. Added shared retirement across backend/journal suspensions and signed-in disposal. Single writer reused; no new backend API/store format. ADR-062 records decision and tradeoff.

- L5: Own checks: shared Android146/iOS147 (0 failures/errors/skips), app build/lint0 and shared AAR/two frameworks passed. Focused AdditionalAccountsScreenTest3/3 passed. Full native78 ran with77 passing and1 failure: IndependentCorrectionChromeTest.statusBarPixelsMatchDrawnAppBarInBothThemes line45, expected dark -16512234 but screenshot white -1. No retry/waiver. Existing iOS Keychain cast warnings remain. No source commit or physical install for this feature.
- L6: Independent fresh scratch comparison: identical splash overlays, same isolated API37 device, baseline66f343e pixel5/5 and candidate5/5 passed. Neither baseline-flake nor candidate causality established. Original full-suite failure remains unclassified. Independent feature S1–S3 PASS:6 baseline common cases identical, candidate14 cases including4 independent probes passed. Risk2/4 retirement/journal/owner boundaries verified; overall PARTIAL because own suite failure unresolved. No extra sweep authorized beyond bound. Needs maintainer decision to open a bounded follow-up for statusbar/screenshot oracle (same-frame appbar/state synchronization is a hypothesis, not proven root cause).
- L7: Preserved scratch evidence/probes for pending follow-up: /tmp/cuenora-additional-verify.DBhdCs and /tmp/cuenora-additional-accounts.NHFxNG. Both emulators stopped; no background checks. Unrelated splash edits preserved. No backend/physical financial writes. Delivery strategy/size exception must still be resolved before commits if over400; no publication authorized.

- L8: User authorized bounded follow-up and proposed appbar/color change as possible cause (S4). First add same-bitmap appbar/status diagnostic while retaining original oracle; run one full native suite to reproduce ordering context. No extra retries beyond bounded correction.

- L9: User-authorized diagnostic full78/78 passed, without a failure tuple; original failure causality still unclassified, not called baselineflake. No appbar/theme production change justified. Added stronger frame-coherent screenshot oracle: wait ONLY for requested appbar content frame, then assert status color in SAME bitmap. Never poll until status matches. Three deterministic oracle regressions cover stale frames, immediate rejection of wrong status, and bounded failure when target theme never appears. Helper compile RED then GREEN; runtime recheck pending. Retained4 independent shared creation probes for durable regression coverage.

- L10: Bounded correction recheck: shared150 Android-host/151 iOS-simulator, nativechrome5/5 and full81/81 (0 failures/errors/skips), app assemble/lint0 passed. Independent targeted source/result readback PASS S4: oracle never polls status color, missingtargetframe fails, same-bitmap assertions retained. S1–S3 prior independent PASS preserved; financial production bytes unchanged. Original failure cause remains UNKNOWN, not called confirmedrace or baselineflake. No production appbar/theme change.
- L11: Parent final app build/lint spotcheck passed. Pre-closure additional-account/oracle authored delta764 lines including tests/docs (splash excluded); ask-on-risk delivery gate before commits requires maintainer exception or chain strategy. Implementation and proof complete locally; source uncommitted and publication authorized within the transaction PR. All independent probes retained as tests, so parent may clean prior external scratch.

- L12: Final local APK installed with adb install-r and MainActivity opened on the authorized Samsung, preserving data. No account was created by the agent; physical creation flow awaits maintainer interaction. Delivery strategy decision still pending, no source commits or publication. All worker scratch cleaned after probes were retained; original statusbar failure cause remains unknown.

- L13: Maintainer replied “Sí” to “¿autorizás una única PR con excepción de tamaño, commits separados por comportamiento y sin publicar todavía?”. Delivery exception approved for this enlargement; local behavior commits authorized, no push/PR creation. Preserve unrelated pending splash work.

- L14: Shared writer/journal/retirement unit committed as78da619 with its common regression tests and ADR062. No splash paths included.

- L15: Frame-oracle unit committed as87ffc31; native entry/host/return-refresh unit with its UI regression tests and execution docs committed ascc16d77. Conventional commits only; no AI attribution. Final passive task-log closure records all identities. Verification ran on the complete working tree including separately pending splash assets/test; no new tests were claimed after commit-only operations. Source units preserve the tested bytes. Original screenshot failure remains unclassified, not claimed as a confirmed appbar defect or baseline flake.
- L16: T1–T4 complete locally. One future single size-exception PR approved; no push, PR or merge performed. Unrelated splash files intentionally remain uncommitted. Final task mirror synchronized; future physical account creation requires maintainer interaction, not agent writes.

- Delivery: The maintainer authorizes including this verified follow-up in the transaction PR against main, separate from the independent splash PR, with the approved size exception. Standalone combined transaction regression suite83 and shared150/151 passed. No merge authorized.
