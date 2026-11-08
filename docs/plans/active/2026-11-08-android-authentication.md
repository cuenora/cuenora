# Execution plan: Android authentication

- **Status:** Verified locally — publication pending
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.02
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer-directed agent

## Goal and user-visible outcome

Sign in or register from native Android through shared repositories. Restore a
stored session, renew rotating credentials safely and sign out. Do not implement
first-account onboarding or imply that authentication grants financial authority.

## Existing state and evidence

The Android host has a branded Compose theme/navigation/components, not auth
screens. Shared code supplies ApiClient, advisory authentication validators and
native pair-level SessionStore implementations. Authentication repositories and
complete network/storage session coordination are missing. Registration accepts
with a generic 202 and no tokens; login does not return an owner identifier.

## In scope

- Shared typed register/login/refresh/logout adapters and deterministic tests.
- Coordinated secure session restoration, rotation and sign-out.
- Native Android forms, session gating and focused interaction tests.

## Out of scope

P11.03 onboarding, financial screens, backend/schema changes, iOS UI,
email verification/password recovery journeys and remote deployment/testing.

## Constraints and invariants

Reuse existing transport, validators and native secure storage without a plaintext
fallback. Never log credentials, decode opaque tokens or invent owner identity.
Preserve registration privacy. Serialize refresh/logout ordering and handle
uncertain rotation without blindly reusing a consumed refresh credential.
Storage failures must not masquerade as successful persistence or logout.

## Steps

- [x] T1: Shared authentication operations and wire-contract tests.
- [x] T2: Session lifecycle coordination and deterministic failure/order tests.
- [x] T3: Android integration, UI tests, documentation and independent verification.

The authoritative specifications and append-only work log are in
[the feature document](../../../odd/tasks/android-authentication.md).

## Acceptance criteria

- [x] Login and generic registration use shared repositories and validators.
- [x] Restoration, renewal and logout respect secure storage and rotation rules.
- [x] Loading, invalid credentials, network and storage failures are visible and safe.
- [x] Existing disconnected navigation remains available only after session gating.
- [x] Shared and Android interaction checks and independent security verification pass.
- [x] Roadmap status reflects observed proof and remaining limitations.

## Verification

Run commands from `mobile/` with the documented JBR and Android SDK. Use an
isolated emulator and synthetic credentials only. Build caches and test outputs
are ignored; any additional scratch must be outside the repository.

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared focused auth tests | RED before implementation, then GREEN | Missing-symbol RED for repository/session/root; 21 auth tests, including six independently authored recovery regressions, GREEN | Passed |
| `./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test` | Nonzero executed tests, no failures | 97 Android-host / 98 iOS-simulator, zero failures/errors/skips | Passed |
| `./gradlew --no-daemon :androidApp:assembleDebug :androidApp:lintDebug` | Build/lint pass; existing warnings recorded | Passed, zero errors and four baseline warnings | Passed |
| `./gradlew --no-daemon :androidApp:connectedDebugAndroidTest` | Auth and existing foundation tests pass on isolated emulator | 19 API37 tests, zero failures/errors/skips | Passed |
| `python3 scripts/check_repository.py` (repository root) | Repository checks pass | Passed after documentation updates | Passed |
| Independent security/concurrency verification | Spec-by-spec result from isolated scratch | S1–S3 PASS; six new probes passed and baseline/candidate 18 testcase outcomes identical | Passed |

Run shared tests separately from Android lint: the foundation record documents a
combined-invocation fixture dependency failure on baseline. No actual hosted API
requests, staging mutation or physical-device proof is implied by these checks.

## Risks, assumptions, and decisions

- Risk: item 2 (credentials/session handling) and item 4 (refresh/logout ordering).
- User explicitly included restoration, refresh and sign-out on 2026-11-08.
- Forecast: 1,200–1,800 authored lines including tests/docs. Maintainer approved one focused PR with a size exception on 2026-11-08;
  no publication is authorized.
- RDD is globally disabled, observed read-only; leave the setting unchanged.

## Completion notes

Own checks and independent security/concurrency verification passed.
Shared allTests and AAR/device-framework/simulator-framework builds also passed.
Five real Android Keystore tests passed on the isolated API37 emulator. No iOS
host changes or repeated Keychain device verification was needed.

The first lint run caught an unused BoxWithConstraints scope; replacing it with
Box fixed the error without suppressions. An auth-to-component test sequence
exposed an IME-sensitive assertIsDisplayed false negative while the error remained
visibly rendered. A scratch screenshot/semantics probe and an IME-dismissed rerun
proved the correction: dismiss the keyboard before retaining the visibility check.
All 19 app tests then passed. No production component was changed for this fix.

Four existing lint warnings remain: OldTargetApi, GradleDependency for Navigation,
DataExtractionRules and MissingApplicationIcon. Existing Native debug-cache/cast
warnings remain visible. The automated implementation checks did not establish
physical-device or hosted API behavior; the later bounded manual smoke below
adds user-observed evidence. API23, timed foreground renewal on hardware, broad
accessibility, process-crash/backup guarantees and financial E2E remain unverified.

Rollback boundary: remove the new shared auth package/tests and native gate/graph,
restore MainActivity/manifest and standalone foundation tests, and retain all prior
P11.01, transport, validation and native secure-storage behavior. No schema or
backend change needs rollback. No push, PR creation or merge has been requested.


## Local work-unit commits

| Unit | Commit | Verification |
| --- | --- | --- |
| Shared authentication contracts | `4b9e3ca` | MockEngine contracts and malformed-success tests |
| Serialized session lifecycle | `98d22da` | Deterministic lifecycle tests plus six retained independent probes |
| Android integration | `17a8eba` | 19 instrumented app tests, build/lint and platform storage proof |

The parent reran the independent probe command with `--rerun-tasks`: six tests
passed, not merely an up-to-date build. Production implementation hashes match
the independently verified scratch snapshot. Baseline/candidate shared validator
and financial-fixture test outcomes matched for all 18 selected tests.

The coherent session unit exceeds the advisory 400-line commit heuristic because
its ordering/recovery tests must accompany its behavior. The maintainer's single
future PR size exception covers the feature; no code or tests were compressed to
meet an arbitrary line target. At implementation closure no hosted backend
operation had been performed; the subsequent authorization below permits only
the bounded manual smoke. No PR or deployment was performed.


## Authorized physical-device smoke — 2026-11-08

The maintainer explicitly authorized installing the existing debug APK and testing
registration/login/restoration/renewal/logout against `https://cuenora.carf.dev`
on their connected Samsung SM-S908B (Android API36). ADB confirmed there was no
prior Cuenora installation, installation succeeded, and Activity start/force-stop
commands completed. Credentials were entered exclusively by the maintainer on the
phone; no passwords, tokens, email addresses or financial records were collected.
No server configuration, administrative access or deployment was involved.

The following are **maintainer-reported screen observations**, not independently
captured UI/network/server telemetry:

| Scenario | Reported result |
| --- | --- |
| Submit test registration | `Registration accepted` appeared; generic acceptance alone does not prove a new account was created |
| Sign in with the same credentials | Home opened and `Sign out` appeared |
| Force-stop and reopen while signed in | Home and `Sign out` returned without entering credentials |
| Online sign-out | Returned to `Welcome back` |
| Force-stop and reopen after online sign-out | Remained at `Welcome back` |
| Sign in again, disable Wi-Fi/mobile data manually, sign out | Returned to login with `Signed out on this device` and the remote-confirmation warning |
| Force-stop and reopen while still offline | Remained at `Welcome back`; no automatic Home restoration |

The implemented restoration path calls refresh before publishing signed-in state,
so the successful reopen exercises launch-time refresh integration. It does not
prove a timer-driven foreground renewal occurred. Offline logout proves the
reported local behavior, not server-side revocation; a server session may remain
until expiry or explicit revocation. No server session was inspected or altered
administratively. The app remains installed and locally signed out. The maintainer
was reminded to restore connectivity; no device-wide settings were changed by ADB.

These results supplement, rather than replace, deterministic tests, isolated
Keystore proof and independent security/concurrency verification. Broader device,
API23, timed-renewal, interrupted-rotation, backup and accessibility evidence are
still outside this manual smoke.
