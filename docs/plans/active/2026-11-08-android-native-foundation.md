# Execution plan: Android native foundation


- **Status:** Active
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.01
- **Tracking issue / PR:** Not created
- **Owner:** Codex

## Goal and user-visible outcome
Implement the Android Compose navigation, branded theme and base components, using existing low/high fidelity references and Android-specific Pen specimens. No functional financial or authentication screens.

## Existing state and evidence
Before P11.01, the Android host showed only SharedModule.name. Existing Compose BOM, Activity and Foundation remain pinned. See [ODD specification and log](../../../odd/tasks/android-native-foundation.md) for exact user requirements, task status and evidence.

## In scope
Android-native design specimens, light/dark theme, licensed fonts, four-destination navigation and accessible reusable controls.

## Out of scope
P11.02–P11.12 functional journeys, backend/auth wiring, iOS changes, remote publishing.

## Constraints and invariants
Follow DESIGN.md and the semantic UI contract. No fabricated financial totals. Minimum 48dp targets, scalable text, system insets and native Back behavior.

## Steps
- [x] Design and save Android foundation in Pen.
- [x] Implement Compose foundation with test-first verification.
- [x] Run app build/lint, emulator tests, repository checks and independent Risk5 verification.

## Acceptance criteria
- [x] Existing design references preserved and new specimens verified.
- [x] Four destinations navigate and restore, including Android Back.
- [x] Theme/components work in light/dark and large text.
- [x] Checks and limitations recorded; no phase-wide completion claim.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Android assemble/lint | Build and no lint errors | Passed; 0 errors, 4 unsuppressed warnings | Passed |
| Compose instrumentation | Navigation, controls, scaling and restoration | 13 API37 emulator tests, 0 failures/errors/skips | Passed |
| Repository checker | Pass | Passed, parent spot check included | Passed |
| Shared Android host tests | Preserve shared behavior | 76 tests, 0 failures/errors/skips in independent scratch | Passed |
| Combined app lint + shared tests | Execute together | Fails implicit fixture dependency; baseline reproduces; separate commands pass | Pre-existing failure |

## Risks, assumptions, and decisions
Risk item 5: AndroidX dependency/test configuration. Native Material3 interactions replace the reference capsule navigation; iOS remains independent. Android destinations are explicitly unwired foundation slots.

## Completion notes
Local implementation and independent verification completed. Final selected-indicator
visual correction and bounded independent delta recheck passed. The maintainer approved a single-PR size exception on 2026-11-08. Local commits are being recorded; publication remains pending.
Authoritative work log: odd/tasks/android-native-foundation.md.

Independent verification found no candidate-caused blocker. It reproduced the
combined lint/shared-test implicit dependency failure on baseline 90a634b and
verified each command separately. No unrelated fixture-generation fix was made.
APK inspection confirmed three fonts and both OFL notices; SDK boundaries remain
23/36/37. Four lint warnings are retained: three baseline scaffold warnings and
GradleDependency for Navigation2.10.2, whose API24 minimum differs from this host.
No TalkBack, physical-device, API23 runtime, iOS or live-backend proof is claimed.
The review switch is globally off (read-only status); no native review started.
