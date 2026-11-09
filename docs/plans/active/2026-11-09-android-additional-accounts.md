# Execution plan: Android additional accounts

- **Status:** Implemented and verified locally; single-PR size exception approved
- **Date:** 2026-11-09
- **Roadmap task ID(s):** P11.03/P11.04 follow-up: additional-account creation gap
- **Tracking issue / PR:** Not created; one future size-exception PR approved, no publication authorized
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Use Add account from Accounts even when an account already exists, with review and safe original-key recovery.

## Existing state and evidence
OnboardingSession.begin only accepts Empty; AccountsScreen has no creation action. Existing authenticated creation API, form validation and encrypted pending-account journal are reusable. See [task specification](../../../odd/tasks/android-additional-accounts.md).

## In scope
Accounts entry, reusable account form/review/recovery, refreshed list on return and signed-in lifecycle retirement.

## Out of scope
Backend/schema/API changes, account editing/archive, transfers, remote publication or real financial writes on a device/backend.

## Constraints and invariants
One account-write coordinator per signed-in subtree and one owner/origin-bound journal. Persist before POST and never replay automatically. Reuse exact money/business dates and established eligibility/ownership checks. Preserve pending splash edits and existing onboarding.

## Steps
- [x] Shared additional-creation intent and retirement with tests.
- [x] Native entry, form chrome and return/refresh with focused tests.
- [x] Own checks and independent verification.

## Acceptance criteria
- [x] S1–S3 verified with deterministic fakes and native UI.
- [x] First-account behavior and retry keys preserved.
- [x] Documentation and verification limitations recorded.

## Verification
| Check | Expected | Observed |
| --- | --- | --- |
| Shared Android/iOS suites | Pass | Final 150 / 151 passed, zero failures/errors/skips |
| App build/lint, native isolated suite | Pass | Final build/lint0; additional focus3; final full81 passed |
| Independent identity/order probes | Pass | Feature probes and targeted screenshot-oracle recheck passed |
| Repository check | Pass | Passed |

## Risks, assumptions, and decisions
Risk items2/4: authenticated writes, durable journal and async state. Reuse the same coordinator rather than two writers sharing one journal. Additional intent must never override an unresolved original command. Single future PR size exception approved; behavior commits, no publication.

## Completion notes
Implementation uses the existing account writer/journal; source remains uncommitted
pending local behavior commits; verification is complete as recorded below. No real-backend/device financial
smoke authorized or performed. Pending splash edits are preserved.

### Historical failed run

The initial own full native suite failed only
`IndependentCorrectionChromeTest.statusBarPixelsMatchDrawnAppBarInBothThemes`
(line45, expected dark background, observed white). Independent focused comparison
passed five baseline and five candidate attempts with identical splash overlays:
this does NOT classify the original failure as baseline flakiness or prove cause.
Feature probes passed S1–S3, including four new independent cases and six identical
baseline common outcomes; the full verification verdict remains partial.

At that checkpoint a maintainer decision was needed for a bounded follow-up.
The proposed frame-coherent oracle was a hypothesis, not a confirmed root cause.
No assertions were weakened or skipped. No source commits, physical update or
publication performed. Shared artifacts built; existing Keychain cast warnings remain.


## Authorized status-bar follow-up and final proof

The maintainer authorized a bounded follow-up, suggesting the earlier color
change as a possible cause. The diagnostic full-suite repeat passed 78/78; no
failure tuple was captured, so the original cause remains unknown. No appbar or
theme production code was changed or blamed.

The test now waits only for the requested appbar content frame, then checks the
status-bar pixel in that same bitmap against the unchanged expected color. It
never waits for a wrong status color to become correct. Three oracle regressions
prove stale-frame handling, immediate mismatch rejection and bounded failure if
the requested theme never appears. Final full81 native cases, chrome5 focused,
shared150/151 and build/lint0 passed. Independent targeted readback confirmed the
assertions are not weakened; earlier feature S1–S3 proof remains valid. Historical
failure and initial partial verdict above are retained, superseded by this current
proof—not by an assertion that baseline flakiness was demonstrated.

Authoring delta is 764 lines excluding unrelated splash edits. The maintainer approved one future PR with a size exception and separate behavior
commits before source commits. No publishing or
real financial writes performed. Existing Native Keychain warnings remain.
