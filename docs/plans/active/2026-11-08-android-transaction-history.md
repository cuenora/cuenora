# Execution plan: Android transaction history

- **Status:** Completed locally; bounded physical smoke confirmed; publication pending
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.06
- **Tracking issue / PR:** Not created; future single size-exception PR approved
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Browse recorded transactions with server-side date, category and account filters.

## Existing state and evidence
Shared listing repository exists; Android Transactions currently exposes recording only. See [task specification](../../../odd/tasks/android-transaction-history.md).

## In scope
Paginated list, existing filters (including accountless), exact stored event semantics, authenticated reads and loading/error/retry states. Preserve recording entry points.

## Out of scope
Currency/type filters, backend changes, editing/voiding, transfer creation, reports, remote publication and physical-device tests.

## Constraints and invariants
No floating-point money, local incomplete-page filtering, totals or planned entries. One transfer row with both amounts. Retire sensitive presentation on sign-out; never save financial state in Bundles.

## Steps
- [x] Authenticated history reader and tests.
- [x] Native screen, filter state, integration and tests.
- [x] Own checks and independent security/ordering verification.

## Acceptance criteria
- [x] S1–S3 in the task specification verified.
- [x] Documentation and tests updated.
- [x] Ownership and async retirement independently checked.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared Android/iOS tests | Pass | 142 / 143; zero failures/errors/skips | Passed |
| Android build/lint and isolated instrumentation | Pass | 53 API37 tests; build passes, lint zero issues | Passed |
| Repository check and independent probes | Pass | Repository/whitespace clean; 9 independent probes and 72 unchanged baseline outcomes | Passed |

## Risks, assumptions, and decisions
Risk items 2 and 4: authenticated reads and asynchronous presentation lifecycle. Default filters unrestricted, explicit refresh, no persistent read cache. Single future PR exception approved; RDD globally disabled.

## Completion notes
Implemented in `f9437ed` (owner-bound reads) and `2ee94ce` (native browsing/filtering); closure retains independent boundary regression tests. One transfer item shows both actual amounts/currencies; posted/voided events and signed openings retain their meaning. All filters are server queries; no local filtering of finite pages. Recording entry remains connected, and return refreshes applied history.

Own review found the account-choice cursor cycle set survived a fresh traversal. The native reopening regression failed before the reset moved before admission, then passed. Final suites include that fix and retained independent owner/retirement probes. Independent verification used fresh baseline/candidate archives and byte-identical production browser host compilation: no blockers for S1–S3. A separate independent emulator/Compose run was not performed; native evidence is the own isolated API37 suite.

Commands from `mobile/` with documented JBR/SDK: separately `./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test`, `:androidApp:assembleDebug :androidApp:lintDebug`, `:shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64`, and `:androidApp:connectedDebugAndroidTest` restricted to an external isolated emulator. Parent spotchecked `:shared:testAndroidHostTest --tests '*Auth*' --tests '*Account*' --tests '*Recording*'`. Shared artifacts passed before final test-only additions. Repository check: `python3 scripts/check_repository.py`.

An initial combined shared-tests/app-lint invocation failed Gradle implicit-dependency validation for existing generated contract fixtures. Established separate command groups all passed without changing build configuration. Do not interpret separate success as success of the combined invocation. Existing iOS Keychain cast warnings remain; no lint issues remain.

No remote publication, backend, hosted, physical-device, minimum-API or complete accessibility proof. RDD globally disabled; assessment is informational, not approval. Risk items 2/4 covered by independent owner-switch/401/403 and suspended success/failure probes. API expansion, currency/type filters, editing and transfer creation remain out of scope.


## Physical smoke follow-up — 2026-11-09
Maintainer confirmed history loads on Samsung SM-S908B/API36, then reported missing
form appbar Back, UUID account labels, misplaced sign-out/header and status-bar
color mismatch. S4–S7 and T4–T6 in the task specification reopen those corrections.
Prior test results remain historical; fresh focused/seam and independent checks
are required before declaring corrections complete. No financial mutations.

### Corrections completed — 2026-11-09

- `489f467`: resolve account labels automatically across paginated metadata for
  source/destination/archived accounts; safe unavailable fallback, retry and fresh
  names on explicit refresh. Keep metadata busy until completion to prevent
  applied filters diverging from rows; deny/retire clears sensitive labels.
- `d13c4af`: appbar Back reuses native discard/busy rules; sign-out resides in the
  drawer without the extra authentication row; consistent theme/statusbar drawing
  and icon contrast. Consumed Scaffold safe-drawing and IME insets keep Cancel
  reachable with three-button/gestural navigation and the keyboard open.

| Correction verification | Observed result |
| --- | --- |
| Full isolated API37 suite, three-button + software IME | 72 passed; zero failures/skips |
| Gestural safe-bottom / IME geometry and action checks | 2 passed, same production bytes |
| App assemble / lint, separate from shared suites | Passed, zero lint issues |
| Independent metadata/reader probes | 9 passed; byte-identical production browser |
| Independent native renewal/logout and statusbar pixels | 2 passed after correcting scratch-only harness assumptions |
| Prior native chrome/form regressions in independent clone | 13 passed |

Baseline regressions reproduced missing names/appbar/drawer and 82.3dp unsafe
Cancel overlap. Parent found and pinned two metadata regressions during readback:
busy ended before a suspended lookup, and refresh reused renamed-account labels.
Both passed RED→GREEN. Independent probes are retained in native tests, followed
by the final 72-case suite. Parent spotchecked shared history tests and the exact
separate app build/lint command. Earlier combined Gradle invocation failure
remains historical and disclosed; no build configuration changed.

The corrected physical Samsung visual smoke is recorded below. Worker tests used
only isolated external emulators with synthetic data, no backend or physical-device
access and no personal financial writes. Existing RDD global-off and single-PR
size exception remain unchanged. Risks 2/4 covered by independent metadata denial,
late read, renewal lifetime and logout evidence. No push or PR created.


Correction regression commit: `a0d85e8`. Physical replacement initially failed with
`INSTALL_FAILED_UPDATE_INCOMPATIBLE` because the installed and new debug signing
certificates differed. No data was cleared by that failed update. The maintainer
then explicitly requested uninstall/reinstall after the data-loss consequence was
explained. Corrected visual observations after fresh sign-in are recorded below.
Future physical APKs must use stable local debug signing, separate from disposable
emulator-test signing state. Never silently uninstall to bypass a signature mismatch.


### Maintainer-confirmed Samsung smoke — 2026-11-09

After the authorized fresh install on Samsung SM-S908B/API36 against the configured
development backend, the maintainer answered “Sí” to each targeted check:

- Cancel is accessible above the system navigation buttons.
- Transactions show account names without opening Filters.
- Sign out is in the side drawer, without the standalone row above the appbar.
- Status-bar/appbar colors match and system icons are readable.
- Form appbar Back returns and confirms discard when changes exist.

The original list load was also maintainer-confirmed. These are user-reported
physical observations, not automated screenshot evidence. This does not establish
physical pagination/filter combinations, logout execution, offline recovery,
all-theme/device coverage or full accessibility. No financial writes were requested
or reported. All five reported correction symptoms are resolved within this smoke.
