# Execution plan: Android alpha actions and design fidelity

- **Status:** Active
- **Date:** 2026-11-15
- **Roadmap task ID(s):** P11.04, P11.06 and P11.10 follow-up gaps; no phase completion claim
- **Tracking issue / PR:** Not created; feature-branch chain approved, publication not authorized
- **Owner:** Maintainer / local coding agent

## Goal and user-visible outcome

Expose safe transaction voiding and zero-balance account archiving, and align the
Android financial experience with the existing editable Pen references. Let the
maintainer inspect incremental APK updates on the connected physical Android.

## Existing state and evidence

Android currently reads transaction rows and account details without their
existing server mutation actions. The API already supports keyed transaction
voiding and account archive/reactivation. See [transaction corrections](../../product/transaction-corrections.md)
and [account management](../../product/account-management.md).

## In scope

The canonical specification, task routes, delivery boundaries and current proof
are maintained in [the feature logbook](../../../odd/tasks/android-alpha.md).
It contains S1–S4 and T1–T4. Keep this execution entry as a locator, not a second
competing task list.

## Out of scope

Backend ledger/schema changes, destructive deletion, automatic financial actions
on the maintainer's device, real credentials in tests, publication and iOS.

## Constraints and invariants

Preserve history, exact money, authenticated ownership and immutable retry keys.
No plaintext retry journals, automatic uncertain-operation replacement, device
uninstall, data clearing, or weakened screenshot/security controls. Use isolated
emulators for automated tests. Native review remains globally disabled.

## Steps

Follow T1–T4 in the canonical logbook; update its proof after each observed result.

## Acceptance criteria

S1–S4 in the canonical logbook define the requested outcomes. Physical feedback
is separate from synthetic emulator proof; neither substitutes for the other.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared Android-host mutation tests | Safe authenticated actions and retry behavior | Not run | Pending |
| Isolated Android instrumentation | Existing journeys plus action/UI regressions | Not run | Pending |
| APK build and Android lint | Installable checked development build | Not run | Pending |
| Repository checks | Documentation and policy consistency | Not run | Pending |
| Targeted physical APK update | Update without data loss or signature replacement | Not run | Pending |

## Risks, assumptions, and decisions

Feature-branch chain keeps main untouched until integrated validation. Exact Pen
screen hierarchy is the reference; justify native deviations in the logbook.
Account removal means policy-compliant archival, not deletion of ledger history.

## Completion notes

Pending. Refer to the canonical logbook for all subsequent evidence and blockers.
