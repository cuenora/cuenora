# Execution plan: Android transfer form and confirmation

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-09
- **Roadmap task ID(s):** P11.07
- **Tracking issue / PR:** Not created; one future size-exception PR approved
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Record an already-completed owned-account transfer with explicit sent/received amounts, business date and confirmation, without executing a bank payment.

## Existing state and evidence
Transfer command/HTTP adapter and financial validation exist. Android RecordingSession, pending journal and modal support income/expense only. See [task specification](../../../odd/tasks/android-transfer-form.md).

## In scope
Existing recording entry points, account selection (including savings), actual dual amounts, optional directional quote and exact informational discrepancy, confirmation and secure original-key retry. Preserve legacy income/expense journals and behavior.

## Out of scope
Backend changes, migrations, bank execution, fetched rates, automatic conversion, editing/voiding, iOS UI, physical-device operations and publication.

## Constraints and invariants
Follow [approved transfer policy](../../product/currencies-and-transfers.md). No floating point. Reuse authenticated backend and encrypted owner/origin-bound journal; never automatically replay uncertain writes or replace their key. New transfer journals are versioned; existing v1 income/expense encodings remain unchanged.

## Steps
- [x] Shared recording, exact quote comparison and compatible journal with tests (shared checks passed).
- [x] Android form/confirmation, entry points and native tests (focused16 passed).
- [x] Own checks, independent risk verification and documentation; local commits recorded below.

## Acceptance criteria
- [x] Same/cross-currency actual amounts and account eligibility validated before review.
- [x] Quote mismatch is informational; actual amounts remain authoritative.
- [x] Recovery preserves both amounts, accounts, date, description, quote and key.
- [x] Existing income/expense and encrypted journal behavior remains verified.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared Android-host/iOS-simulator suites | Tests pass without skips | Final independent snapshot: 163 host / 164 simulator, zero failures/errors/skips | Passed |
| Android assemble/lint, shared artifacts | Builds pass, diagnostics recorded | Both APKs, Android AAR and two iOS frameworks passed; final parent lint zero issues | Passed |
| Explicit isolated-emulator instrumentation | Focused transfer/Keystore and full native suite pass | Focused16/16 passed; final exact-candidate full98/98 passed under both host GPU and SwiftShader | Passed |
| Independent scratch-copy risk probes | No change-caused blocker | Financial/original-key/owner-switch/oracle/legacy differential probes passed; final input-matched uninstrumented candidate full98 passed in both renderers | Passed |
| Repository checks | Pass | python3 scripts/check_repository.py and git diff --check passed | Passed |

## Risks, assumptions, and decisions
Risk items 1/2/3/4: persisted journal compatibility, authenticated shared flow and asynchronous retry lifecycle. One future size-exception PR approved. RDD globally off; do not enable. Existing entry points expose Transfer in the shared modal; account-details context preselects its source. No new dependencies.

## Completion notes
P11.07 is implemented and locally verified. Final shared suites pass163 Android-host/164 iOS-simulator with zero failures/errors/skips. All14 RecordingTest and2 encrypted-journal cases pass. Both APKs, shared AAR/two iOS frameworks and final lint (zero issues) passed. Parent repository validation and diff checks passed. Independent wire/key/owner-switch, exact-decimal oracle and legacy recovery probes were adopted as regression tests; baseline differential preserved income/expense output and journal bytes.

### Native test investigation and limitations
Earlier software-rendered full-suite runs failed one unchanged synthetic splash callback assertion (`StartupSplashTest:62`), while baseline93 and isolated splash6 passed. A debug-only invalidation experiment failed and was reverted. A later scratch-only logging run passed98, but was not equivalent-candidate proof. No splash production/test fix, assertion relaxation or timeout extension was retained.

The final controlled comparison built the **exact uninstrumented candidate**: all135 mobile source/config/resource inputs matched live before and after. The same APKs passed all98 native tests under host GPU (147.59s) and SwiftShader (230.218s), on SDK37 read-only isolated emulator sessions through explicit serial-bound adb. This supplies clean final native proof, but does not explain the historical intermittent failure or establish renderer causality. Preserve that limitation rather than claiming a fix.

Input manifest SHA256: `42107ebd43e95333659006710832846c5e9a7c1cd6a7474c4549d3d187477c89`.
App APK: `f0d5689588dfcb948ba0678e56d87de2d875757dcbb1d0dd8e91fd9495591d5e`.
Test APK: `1d963fe8cf63b10f8959740def92a35627a639ce9a41077b78533de494b64ae5`.

### Device-isolation incident
A diagnostic run incorrectly used Gradle connected-device discovery despite the explicit restriction. Logs list only its emulator, but the selected-device assurance was insufficient. Operations stopped; maintainer separately authorized a read-only audit. Samsung SM-S908B/API36 user0 still has Cuenora installed with first/last-update time2026-11-09 19:16:31, predating diagnosis and matching the earlier authorized installation. No indication of a later reinstall/update; application data was not inspected or guaranteed. The final controlled proof used explicit qemu-verified serial-bound adb only. No feature installation, app-data operation or publication performed.

### Delivery
Branch `feat/p11-07-transfer-form`; one future size-exception PR approved. Local commit identities recorded in the task specification. Physical feature smoke, hosted checks, minimum-API runtime, iOS UI and full accessibility proof remain outside this local evidence. No Phase11 completion claim.

Local evidence initially at `/tmp/cuenora-p1107-evidence.5h1v0byx`; final controlled raw results/hashes will be retained there after scratch cleanup.

Implementation commit: `3c4106e`. Parent APK-build spot check passed; all135 mobile input hashes remained equal after commit. Final controlled logs and diagnostic-only sources retained under `/tmp/cuenora-p1107-evidence.5h1v0byx`; large scratch builds/emulator copies removed.
