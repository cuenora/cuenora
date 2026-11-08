# Execution plan: Android first-account onboarding

- **Status:** Active — locally implemented and verified; publication pending
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.03
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

A signed-in Android user can verify email, explicitly confirm a profile time zone,
review a normal/savings account and save it with an optional dated opening balance.
Existing users are not forced through first-account creation.

## Existing state and evidence

At the starting baseline `e426303`, P11.02 was integrated and Android entered
the foundation shell after authentication without financial resource wiring.
Verification request/confirmation, profile-zone PATCH and account list/create
already existed in the backend. There was no profile GET/status API. The shared
account adapter expects an owner snapshot that login does not provide; do not
invent an authenticated owner to reuse it.

## In scope

- Native verification request/resend and same-account confirmation or status recheck.
- Explicit profile-zone confirmation and first-account form/review/save.
- Exact opening amounts/dates, server gates and idempotency-safe recovery.
- Shared/native tests and independent security/concurrency verification.

## Out of scope

Password recovery, global sign-out, backend identity redesign, account details,
income/expense entry, dashboard statistics, hosted testing and publication.

## Constraints and invariants

Follow the [feature specification and logbook](../../../odd/tasks/android-onboarding.md)
for verbatim scope, task state and delivery decisions. Reuse existing financial
rules and Compose components. Do not treat mail acceptance as verification,
authentication as financial authority, or a failed list as an empty account list.
Never parse money through binary floating point. Preserve one logical mutation
across uncertain retries, session changes and interrupted UI lifetimes.

## Steps

- [x] 1. Implement and test authenticated onboarding transport/session boundaries (T1).
- [x] 2. Connect native first-account onboarding and document behavior (T2).
- [x] 3. Run own checks and independent security/concurrency verification (T3).

## Acceptance criteria

- [x] S1–S4 in the feature specification are covered with observed proof.
- [x] Verification and date/zone gates remain server enforced.
- [x] Cancel, denied access, failed load and uncertain write states are explicit.
- [x] Existing session/native-navigation behavior is preserved.
- [x] Applicable shared tests, build/lint, instrumentation and independent probes pass.

## Verification

Run Gradle from `mobile/` with the documented JBR/SDK and an external temporary
`ANDROID_USER_HOME`. Use only an isolated emulator and synthetic repositories.

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 scripts/check_repository.py` (root) | Repository consistency | Passed; see final evidence below | Passed |
| `./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test` | Shared tests pass on both targets | Passed; see final evidence below | Passed |
| `./gradlew --no-daemon :androidApp:assembleDebug` | App builds | Passed; see final evidence below | Passed |
| `./gradlew --no-daemon :androidApp:lintDebug` | No lint errors; report retained warnings | Passed; see final evidence below | Passed |
| `./gradlew --no-daemon :androidApp:connectedDebugAndroidTest` | Isolated Compose tests pass | Passed; see final evidence below | Passed |
| Independent security/concurrency/idempotency probes | Candidate meets S1–S4, existing behavior preserved | Passed; see final evidence below | Passed |

## Risks, assumptions, and decisions

- Security and session ordering require independent verification after own checks.
- Add a minimal authenticated profile read to obtain server-derived owner/verification/zone; no dependency or migration change.
- Native review is globally disabled; keep it disabled.
- Forecast 1,200–1,800 authored lines exceeds the review budget; single-PR size exception approved; separate behavior commits.
- This request does not authorize hosted access, physical-device use or publication.

## Completion notes

Implementation, own checks and independent security/concurrency verification completed. Branch `feat/p11-03-first-account-onboarding` created from
`e426303`; profile read, shared coordinator/journal and native UI implemented. Keep P11.03 unchecked.

### Own verification evidence — 2026-11-08

- Shared Android-host/iOS-simulator suites: 107/108 tests, zero failures/errors/skips.
- App debug/instrumentation APK builds passed; lint: zero errors, four baseline warnings.
- Fresh disposable API37 emulator: 24/24 app tests passed after fixing the new
  test helper to click dialog buttons without trying to scroll them.
- Backend formatting and Clippy passed; all three listed disposable PostgreSQL
  harness targets passed, including four profile tests and unchanged opening behavior.
- Cargo workspace failed at the unchanged healthcheck listener test with WouldBlock;
  the same failure was reproduced at baseline; independent full Cargo workspace passed.
- Repository check and `git diff --check` passed during implementation.

### Final verification and work units

- `93b09ba`: authenticated profile read, OpenAPI, self-only/no-write PostgreSQL coverage.
- `b806754`: shared authorized requests, onboarding coordination, encrypted journal,
  exact-command retry and owner/isolation/tamper regression tests, ADR-058.
- `89a7084`: Android first-use/verification/zone/form/review/recovery UI and Compose tests.
- Final shared suites: **109 Android-host / 110 iOS-simulator**, no failures/errors/skips.
- Final app suite: **25 API37 tests**, no failures/errors/skips. Fresh disposable
  emulator only, stopped after testing. Debug/instrumentation APK builds and lint passed.
- Final isolated PostgreSQL profile harness: **5 passed**. Earlier opening-balance
  and old-key migration harness targets passed. Ordinary Cargo does not run these
  ignored database cases.
- Independent candidate full Rust workspace passed. Baseline `e426303` reproduced
  the initial intermittent healthcheck listener WouldBlock at line 43 / join panic
  at line 85. This is a baseline advisory; no unrelated fix was introduced.
- Independent verdict: **PASS S1–S4**, including real auth owner switching and 401,
  canceled/concurrent/restarted writes, immutable key/command preservation,
  authenticated profile GET/HEAD invariance and encrypted journal tampering.
- Parent spot checks: final five-test profile harness, format/Clippy, shared suites,
  full native suite and repository validation passed.

The four existing lint warnings and Kotlin/Native toolchain/debug-cache warnings
remain visible. RDD was globally disabled throughout; no receipt is claimed.
The single-PR size exception was explicitly approved, but there is no publication,
push, PR, merge or deployment authorization. Before hosted Android testing, deploy
the backend with `GET /v1/me`; no hosted mailbox/backend or physical-device smoke
was performed. API23, full accessibility and real-backend mobile E2E are not proven.
Keep this plan in `active/` until integration; checking the roadmap means local
implementation/proof, not production readiness.
