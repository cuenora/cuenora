# ADR-062: One account writer for onboarding and additional accounts

- **Status:** Accepted; locally implemented and verified
- **Date:** 2026-11-09
- **Related:** [ADR-058](058-first-account-onboarding.md), [execution plan](../../plans/active/2026-11-09-android-additional-accounts.md)

## Context
The first-account coordinator intentionally only exposes begin from an empty
owner. Accounts browsing does not expose creation, leaving no way to add a second
account. The authenticated API and exact account/opening command already exist.
The encrypted pending-account journal stores one immutable command/key per owner
and backend origin; it is not a collection of drafts.

## Decision
Reuse one signed-in OnboardingSession for both entry points. The original begin
retains its first-account guard. Explicit beginAdditional refreshes the profile
and reads pending recovery before offering a new form, even when accounts exist.
Unknown operations always enter original-key recovery, never a replacement form.

The additional native host owns only memory-only visibility, not a second writer.
It reuses the existing form/review/verification components with neutral account
copy, appbar/system Back, discard confirmation and safe system/IME insets.
Returning refreshes the account list and history. Cancellation does not clear a
pending write. Acknowledged saves clear their journal; a later creation gets a
fresh operation key.

Disposing the signed-in subtree retires the shared coordinator. Guards around
suspension points prevent late publication and prevent a POST after a delayed
journal save completes following retirement. Retirement does not delete recovery
storage. This also protects the original onboarding path using the same writer.

## Alternatives and consequences
A separate creation coordinator would isolate presentation naming, but would
introduce two writers sharing a single journal and require additional locking or
a storage redesign. Reusing one coordinator avoids that race and keeps the
existing wire contract, journal format and native storage adapters unchanged.
Tests must preserve first-account behavior and pin the explicit additional intent,
original-key recovery and retired-state guards. No backend changes are required.
