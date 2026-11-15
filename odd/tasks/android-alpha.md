# Android alpha: safe actions and faithful financial screens
Branch: `feat/android-alpha`; base: `eee983a8aa7f35715312476887a5fb7af7fc90b1`. Delivery: auto-chain, feature-branch-chain; local work only, no publishing authorized.
Runner: mobile Gradle Android-host tests, isolated emulator instrumentation, APK/lint and repository checks. Forecast: 1,500–2,500 authored changed lines; slice by behavior, not file type.

## Specs

- **S1** — "La ui android no permite cancelar transacciones en las que el usuario se haya equivocado o que al final no se hayan completado y debería permitirlo." Clarification: asked whether this means "anular una transacción ya registrada y deshacer su efecto en el saldo, conservando su historial"; user answered "Sí". Expose explicit confirmation and existing keyed void API, retain history and authoritative balances; no backend ledger-policy changes. Check success, cancellation without writes, duplicate/busy protection, unavailable/unknown results and safe original-operation retry.
- **S2** — "La ui android no permite eliminar alguna account a pesar de que haya dejado su saldo a 0." Existing approved policy permits archive at exact zero, retaining history, rather than destructive deletion. Expose archive with clear naming/confirmation and server-enforced eligibility; nonzero/rejected operations must not claim success. Preserve archived history and allow existing reactivation policy.
- **S3** — "Y lo más importante de las 3 es que la UX es un desastre, mucho texto, no hay tarjetas, y no sigue para nada el diseño de baja fidelidad que hemos creado en pen. Y debería de seguirlo al pie de la letra excepto por alguna decicion especifica o para mejorar la UX de los usuarios de una plataforma específica o sea, tal vez un cambio específico en android porque los usuarios están más acostumbrado a ese flujo etc. Pero no el desastre que tenemos ahora." Compare actual financial screens against `docs/cuenora-design.pen`; use its hierarchy/grouping/navigation/primary actions and reduce always-visible explanatory prose without hiding financial meaning. Record justified native deviations. Preserve English UI, accessibility and exact amounts. Verification includes reference/frame comparison and native synthetic UI tests, with maintainer physical-device feedback.
- **S4** — "Me gustaría mientras desarrollamos ver los cambios en caliente de la app en el android físico que tengo conectado por adb". Build and update APK on the explicitly connected physical Android, never uninstall/clear data/change signing to bypass conflicts. No autonomous sign-in or financial operations; no device instrumentation suites, private screenshots or credential collection. Iterative rebuild/install is not a claim of instant hot reload.

## Tasks

- **T1** — S1; delegated then parent-integrated, awaiting independent proof; parallel independent transaction action unit with disjoint account surfaces; parent owns application/Activity integration; commit pending. Risk: items 2/4 (authenticated mutation, concurrency and durable retry).
- **T2** — S2; delegated then parent-integrated, awaiting independent proof; parallel independent account action unit with disjoint transaction surfaces; parent owns application/Activity integration; commit pending. Risk: items 2/4 (authenticated mutation and in-flight lifecycle).
- **T3** — S3; inline, initial preview implemented / full fidelity pending; source reference read from Pen MCP, existing native screen tests; commit pending. Risk: item 3 if shared chrome/helpers affect unrelated existing screens; independent verification once at feature end.
- **T4** — S4; inline targeted physical APK update, pending; build/long suites delegated; no financial/device test automation. Commit not applicable.

## Log

- **L1 — Original request (2026-11-15)**

Hemos culminado la fase 12 del plan maestro pero hay 3 cosas importantes que he encontrado para dar la interfaz android siquiera por alfa.

1. La ui android no permite cancelar transacciones en las que el usuario se haya equivocado o que al final no se hayan completado y debería permitirlo.
2. La ui android no permite eliminar alguna account a pesar de que haya dejado su saldo a 0.
3. Y lo más importante de las 3 es que la UX es un desastre, mucho texto, no hay tarjetas, y no sigue para nada el diseño de baja fidelidad que hemos creado en pen. Y debería de seguirlo al pie de la letra excepto por alguna decicion especifica o para mejorar la UX de los usuarios de una plataforma específica o sea, tal vez un cambio específico en android porque los usuarios están más acostumbrado a ese flujo etc. Pero no el desastre que tenemos ahora.

Me gustaría mientras desarrollamos ver los cambios en caliente de la app en el android físico que tengo conectado por adb

- **L2 — Clarification:** User answered "Sí" to posted-transaction voiding preserving history and reversing balance effect.
- **L3 — Delivery:** User answered "Perfecto" to feature-branch chained PRs, physical Android iteration and leaving main intact until integrated validation. No push/PR creation authorized yet.
- **L4 — Existing evidence:** Clean main at baseline above. Transaction repository exposes void; Android only renders rows. Account API supports archive/reactivation, exact zero required server-side. Phase11 is Android; Phase12 iOS remains unchecked. Pen Home BF49A and account detail InPc2 inspected. Native review mode is globally off; do not start RDD.

- **L5 — Implementation and own checks:** Added authenticated void orchestration and encrypted original-key recovery; account status actions with explicit confirmation and unknown-result refresh; production wiring refreshes financial views. Parent repaired incomplete worker changes after both writers stopped on a usage limit. Shared Android-host suite: 243 tests, zero failures/errors/skips; Android app/test Kotlin compilation passed. Repository validation and whitespace checks passed. Archive behavior RED was not observed (the first test attempt was blocked by concurrent incomplete code); new native acceptance tests await execution.
- **L6 — Observed RED/GREEN:** Baseline HomeFidelityTest ran on an isolated API37 emulator: 1 test failed at missing balance-summary semantics. Parent void recovery regressions: 8 tests, 2 failed (recovered cancellation and clearing a prior journal after 4xx), then all 8 passed after repair.
- **L7 — Visual preview scope:** Home now leads with currency-separated balances and grouped per-account planning; date editing is disclosed on demand. Account/transaction records and Insights metrics have meaningful surfaces. Compact native navigation uses Activity as the short Transactions label at ordinary phone widths; below 360dp or enlarged text uses the existing list. Removed ready-Home's duplicated persistent destination strip. Full S3 remains open: individual upcoming payments and deeper recording/recurrence/savings reference alignment are not implemented.
- **L8 — Independent proof:** First fresh-snapshot run stopped before tests due Gradle default heap/metaspace GC thrashing, not a demonstrated application defect. Authorized one command-line-memory-only rerun of the same immutable snapshot; results pending. Assessment refused untracked inventory (risk high/unassessable); no RDD lifecycle started because the global mode is off.
