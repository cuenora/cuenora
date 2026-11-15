# Execution plan: Android alpha actions and design fidelity

- **Status:** Latest input/chrome changes verified and installed; broader Pen fidelity remains in progress
- **Date:** 2026-11-15
- **Roadmap task ID(s):** P11.04, P11.06 and P11.10 follow-up gaps; no phase completion claim
- **Tracking issue / PR:** [#147–#160](https://github.com/cuenora/cuenora/pull/147) merged on 2026-11-15; maintainer approved publication without an issue and the 490-line controls exception
- **Owner:** Maintainer / local coding agent

## Goal and user-visible outcome

Expose safe transaction voiding and zero-balance account archiving, and align the
Android financial experience with the existing editable Pen references. Let the
maintainer inspect incremental APK updates on the connected physical Android.

## Merge reconciliation — 2026-11-15

GitHub readback confirms every PR in the [#147](https://github.com/cuenora/cuenora/pull/147)
through [#160](https://github.com/cuenora/cuenora/pull/160) chain is merged.
Local `main` matches the fetched `origin/main` at `cfa6815`. Earlier sections and
log entries describing local-only work or blocked publication are historical;
this reconciliation supersedes their delivery status, not their test evidence.

The merge does not close full S3 Pen fidelity or physical keyboard/layout
feedback, nor does it imply a new full-suite run. Keep this plan active.
No implementation, device actions or tests were performed for this documentation update.

## Initial state and evidence

Android currently reads transaction rows and account details without their
existing server mutation actions. The API already supports keyed transaction
voiding and account archive/reactivation. See [transaction corrections](../../product/transaction-corrections.md)
and [account management](../../product/account-management.md).

## In scope

The canonical specification, task routes, delivery boundaries and current proof
are maintained in [the feature logbook](../../../odd/tasks/android-alpha.md).
It contains S1–S12 and T1–T9. Keep this execution entry as a locator, not a second
competing task list.

## Out of scope

Backend ledger/schema changes, destructive deletion, automatic financial actions
on the maintainer's device, real credentials in tests, new releases and iOS.

## Constraints and invariants

Preserve history, exact money, authenticated ownership and immutable retry keys.
No plaintext retry journals, automatic uncertain-operation replacement, device
uninstall, data clearing, or weakened screenshot/security controls. Use isolated
emulators for automated tests. Native review remains globally disabled.

## Steps

Follow T1–T9 in the canonical logbook; update its proof after each observed result.

## Acceptance criteria

S1–S12 in the canonical logbook define the requested outcomes. Physical feedback
is separate from synthetic emulator proof; neither substitutes for the other.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared Android-host mutation tests | Safe authenticated actions and retry behavior | 243 passed, zero failures/errors/skips | Passed |
| Isolated Android instrumentation | Existing journeys plus action/UI regressions | Initial app122/125 plus corrected archive4/4; shared native9/9; no full125 rerun | Passed (bounded) |
| APK build and Android lint | Installable checked development build | APKs built; lint XML0 issues | Passed |
| Repository checks | Documentation and policy consistency | Validation and whitespace checks passed | Passed |
| Targeted physical APK installation | Authorized profile, no uninstall/clear/signing workaround | User0 initially absent; verified APK installed successfully and package presence confirmed | Passed (installation only) |

## Risks, assumptions, and decisions

Feature-branch chain keeps main untouched until integrated validation. Exact Pen
screen hierarchy is the reference; justify native deviations in the logbook.
Account removal means policy-compliant archival, not deletion of ledger history.

## Completion notes

The verified initial preview is preserved in six local child branches above the
tracker, ending at `194aab7`; nothing is published. The verified APK is now
installed in the explicitly authorized personal profile on the Samsung. Corrected archive
assertions pass4/4 and secure-store native tests pass9/9. Home's synthetic capture
shows no clipping, but it is not full app-chrome or full Pen-fidelity proof.

The initial unscoped package lookup was denied access to secondary user150.
After explicit maintainer authorization, user0 lookup found no installed package;
the verified APK was installed there and its presence confirmed. No app launch,
credentials or financial operations were performed. See L15–L19 in the logbook.

## Follow-up UX and device incident

S5–S8 add drawer-free settings/logout, locale/profile-zone snapshot formatting and
prefilled explicit recurring-payment review. Native136 yielded135 passes and one
field/Back test failure reproduced on the baseline. All11 new cases pass. After
the observable-locale correction, focused timestamp/planning8/8 and lint0 pass.
Source slices52b30e9/6282f40/c810f25 are local; nothing is published.

The next physical update did not reach installation: the same Samsung's personal
profile no longer lists Cuenora, although prior installation/presence was confirmed.
Cause and local data integrity are unknown. Emulator-only result names do not prove
Gradle discovery/cleanup stayed isolated, and retained logs lack complete tracing.
No direct uninstall/clear command was found; no cause is asserted. All device
mutations are stopped while asking about any manual installation/profile changes.
See L26–L30 for exact scope, proof limitations and retained evidence.

### Resolved installation incident and current follow-up

The maintainer confirmed manual uninstallation after backend sign-in failed.
Operator-only diagnostics identified stale exact proxy trust; a later anonymous
malformed-login check returned the expected400 instead of503. The verified APK
was reinstalled and launched in explicitly authorized Samsung user0, and the
maintainer confirmed sign-in works. The preceding incident section is historical,
not an active installation blocker.

S9–S12 now cover Pen's plus-before-settings action order, timestamp without a
visible zone suffix, reduced redundant prose, semantic keyboards, calendar date
selection and exact comma/dot decimal normalization at the Android input boundary.
Existing financial validation, business-date policy and explicit submission remain
unchanged. T8/T9 scoped checks passed; see the final verification record below. No automated
tests may run on the physical phone or use Gradle connected-device discovery
while it is attached.

## Native inputs and latest physical preview

S9–S12 are integrated through merged PRs #147–#160, originating in slices `ef6213b`, `edc83a2`, and `a7582f9`.
The reusable-control slice is490 authored lines; code and its native/security
regressions were kept together rather than minified or split by file type. The
maintainer approved the size exception and publication without an issue;
[PR #157](https://github.com/cuenora/cuenora/pull/157) was merged with `size:exception`.

- Shared Android-host:243 passed; shared/backend280 source files unchanged.
- Native affected paths:29/29 passed; final test-only LocalActivity correction
  retained production bytes and privacy1/1 passed. Final lint XML:0 issues.
- Historical full146 was124 passed/22 date failures before correction. It was
  **not rerun**; targeted checks cover all22 original failures plus7 native
  regressions. This is bounded proof, not a full-suite PASS claim.
- Normal-signing APK built with unchanged certificate and196 source hashes
  matching the independent snapshot. Samsung user0 install-r and launch succeeded.

IME-driven viewport movement was reproduced in synthetic tests; general date
setup now uses the public calendar action, while an independent real-body-touch
case explicitly waits for a visible IME/stable viewport. No keyboard hiding or
arbitrary sleeps hide the condition. Native dialog callbacks use current form
handlers, and both windows explicitly inherit the parent secure policy before
showing. No backend/domain/dependency change was required.

Physical keyboard/layout feedback and full S3 Pen fidelity remain pending.
No phone tests, private screenshots, credential entry or financial submission
were performed.
