# Execution plan: Android alpha actions and design fidelity

- **Status:** Accepted fidelity work published as PRs #163–#167; CI/integration remain separate
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
It contains S1–S17 and T1–T14. Keep this execution entry as a locator, not a second
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

Follow T1–T14 in the canonical logbook; update its proof after each observed result.

## Acceptance criteria

S1–S17 in the canonical logbook define the requested outcomes. Physical feedback
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

## Home-only aesthetic follow-up — 2026-11-16

The maintainer confirms functionality works and requests one screen at a time, starting Home. Low-fidelity Pen screens exclusively define structure; the design system defines typography/colors. The three high-fidelity examples are not structural references. See S13/T10/L47–L48 in the canonical logbook. No functional, backend or shared-chrome changes are in scope. Physical acceptance of the animated startup is recorded separately in its completed plan.

Home follow-up checks: app/test APK assembly passed, but isolated ADB transport disappeared before installation, so no native test or visual proof was produced. Production Home remains unchanged. The new synthetic regression tests are preserved; no repeated emulator recovery or phone operation is authorized by this status. See L49–L50 for evidence and the pending decision.

The maintainer updated Android Studio/SDK and opened emulator-5554. Explicit virtual-device checks and both APK installs succeeded; the two new tests then produced expected baseline RED. The parent implemented Home-private composition only; GREEN and full-suite results are pending. The earlier unavailable-runtime note is historical, not a new request for permission.

Home composition verification: expected baseline RED2/2, final focused11/11, build/test APKs and lint0 passed. Full candidate155had7native calendar failures; exact baseline153reproduced the same7failures. No failures were suppressed or repaired outside scope, and no second candidate full-suite run is claimed. Synthetic390x844light and320x844dark/large-text captures were inspected. The open balance row, planning card/shortfall cue, compact no-pending rows and upcoming region now use the low-fidelity hierarchy with existing theme tokens. Existing recurrence routing remains; individual occurrence rows, a dedicated Savings destination shortcut and redundant recording shortcuts were not invented by adding new flows to this aesthetic-only unit. Full S3 and maintainer acceptance remain open.

## Home physical update — 2026-11-16

The maintainer authorized installation on the Samsung and conditional removal only for a signing conflict. Normal-signing build passed; explicit personal-user0 update returned Success without uninstalling or clearing data, and package presence was confirmed. The normal debug signer is unchanged. No app launch, sign-in, financial action or phone test was performed. See L58–L59 for artifact/proof details. Maintainer visual acceptance and full S3 remain open.

## Accounts overview aesthetic iteration — 2026-11-16

The maintainer accepted the Home iteration and authorized Accounts overview next. Use low-fidelity PDaOP for structure and existing design-system styling. Only the list/private row presentation is in scope; preserve account detail, creation, archived visibility, exact/partial totals, pagination and existing callbacks. No new data or financial flows. See S14/T11/L60–L61 in the canonical logbook. Full S3 remains open.

Accounts baseline APKs compiled, but the explicit emulator update was refused for a signing mismatch before instrumentation. Production Accounts is unchanged; no RED/GREEN claim. New synthetic tests and external snapshot are preserved. Removing emulator app/test packages requires explicit consent; no phone operation or emulator uninstall was performed.

The maintainer explicitly authorized removing both Cuenora app/test packages only on emulator-5554. Parent confirmed the virtual target and both uninstalls returned Success; Samsung was untouched. Resume baseline tests with the stable normal debug signer. This supersedes the earlier consent blocker, not its evidence.

Accounts overview proof: expected RED2/2; app/test builds and lint0 passed. Initial focused18had17passes and one duplicate-balance selector failure; the row-scoped exact-balance assertion then passed its single-method recheck. Full157had149passes/8failures:7previously reproduced baseline DatePicker failures and that corrected assertion; no full-suite rerun is claimed. Parent verified source-byte agreement, unchanged selected-account detail branch and3synthetic light/dark/large-text previews. The list now uses open rows/dividers/chevrons, per-currency totals, existing exact amount formatting and design-system fonts/colors. Native add/archive/refresh controls and pagination are preserved. No new pending-data read or Savings flow was introduced. Samsung was not updated during this iteration. Maintainer Accounts visual acceptance and full S3 remain open.

## Accounts physical update — 2026-11-16

The maintainer requested Samsung update with reinstall fallback if necessary. Normal-signing build passed6s, signature verified, and the explicit personal-user0 update returned Success; post-install package presence was confirmed. No uninstall/data clear was needed, and no app launch, login, financial operation or phone test was performed. The prior “Samsung not updated” note is historical. Source hashes still match verified Accounts code. See L70–L71 for artifact/proof details; maintainer Accounts visual acceptance remains open.

## Account-detail aesthetic iteration — 2026-11-16

The maintainer responded positively after the Accounts update and authorized account detail next. Low-fidelity InPc2 defines structure; existing design-system styling and functional behavior remain authoritative. Scope is detail-only name/balance/information/actions. Do not add missing illustrative history/planning data or new recording flows. Preserve list, creation, existing callbacks and archive/reactivate safety. See S15/T12/L72–L73 in the canonical logbook.

Account-detail iteration is partial: its two new fidelity tests pass, but the bounded compatibility recheck still has one legacy type-label assertion mismatch in AccountsLowFidelityTest. The full native suite was not run; lint predates the last metadata adjustment. Parent compile/list-byte-preservation checks passed. No installation/publication/completion claim. A further presentation-only test correction in that specific file requires maintainer approval; see L74–L75.

The maintainer approved the specific presentation-assertion correction in AccountsLowFidelityTest.kt and pending verification. Preserve data/callback coverage and production bytes; no additional functionality or automatic new-finding correction. This supersedes the earlier decision blocker, not its test evidence.

Authorized account-detail final proof: original-blocker4/4, app/test builds and final lint0 passed. First full159had152passes and exactly7previously reproduced baseline DatePicker failures; no new failure or suppressed case. Parent build/source-byte/list-preservation checks passed. This supersedes earlier partial compatibility notes, not their history. Detail now emphasizes the account name and open balance before existing actions, with compact metadata; no new history/planning/financial flow. Samsung has not been updated for this detail iteration; maintainer visual acceptance/full S3 remain open.

## Account-detail physical update — 2026-11-16

The maintainer explicitly requested Samsung update. Normal-signing build passed4s, signature verified, and the personal-user0 update returned Success with package presence confirmed. No uninstall/data clear, app launch, sign-in, financial action or phone test. The verified four source/test hashes are unchanged. This supersedes the earlier no-Samsung-update status for this iteration. See L81–L82; maintainer detail visual acceptance/fullS3 remain open.

The maintainer reported the detail Back control differs from low-fidelity Pen. Parent verified jiJIC has a leading arrow-left plus Accounts, while the delivered control has only Back to accounts text. T13/S16 correct that specific discrepancy and preserve native accessibility/Back behavior; no new functional scope.

Back control now follows jiJIC: leading arrow and Accounts label,20dp icon/8dp gap, native48dp minimum after focus-ring padding. Baseline RED observed; final Back geometry/callback/no-reload recheck1/1 and build/lint0 passed. Initial full160had7known DatePicker failures and the minimum-target assertion subsequently corrected/rechecked; no full rerun claim. Parent inspected final synthetic Back PNG and verified all other detail/list content unchanged. Samsung update/maintainer acceptance remain pending for this correction.

Corrected Back preview was updated on Samsung after explicit maintainer approval. Normal build4s/signature verification passed; personal-user0 update returned Success and package presence was confirmed, without uninstall or clearing data. Source/test hashes remain verified. No phone launch/login/financial operation/test; visual acceptance remains pending. See L89–L90 for artifact proof.

### Account-form fidelity (T14)

Low-fidelity `a6mwI` now guides the creation form: Accounts Back row, Add account
heading, stacked type/currency selectors, optional opening fields and existing
review/cancel flow. Native theme and first-account compatibility are retained.

Final account-form/onboarding/additional tests passed 10/10; shared host tests
passed 243/243. Final APK builds and lint passed (zero lint issues), as did
independent compatibility verification and parent build/source readback.
The initial full candidate run had 8 failures in 162 tests. Seven DatePicker
failures reproduce on the exact baseline; one manual-date failure did not
reproduce in the independent baseline full run or final candidate focused run.
The later authorized investigation passed the exact manual-date test both isolated
and in a full 162-test run (155 passed, the same seven baseline calendar failures).
Its original one-off cause remains unknown; no date fix or globally green suite
is claimed. The Samsung update succeeded and the maintainer accepted the form.
Current-session gh publication is authorized without linked issues; merge is not.
See T14/L93–L101 for proof, acceptance and delivery boundaries.

### Published fidelity chain

[Draft tracker #163](https://github.com/cuenora/cuenora/pull/163) targets `main`.
Review children in order: [Home #164](https://github.com/cuenora/cuenora/pull/164),
[Accounts #165](https://github.com/cuenora/cuenora/pull/165),
[detail/Back #166](https://github.com/cuenora/cuenora/pull/166), then
[creation form #167](https://github.com/cuenora/cuenora/pull/167). Each child
targets its immediate predecessor; all review slices are below 400 changed lines.
The maintainer approved publication without linked issues. No merge or aggregate
tracker size exception is authorized; reassess before eventual integration.
