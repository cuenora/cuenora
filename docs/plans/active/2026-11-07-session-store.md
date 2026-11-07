# Execution plan: Native secure session storage

- **Status:** Active
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.07
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Provide shared SessionStore read/replace/clear operations backed by Android
Keystore encryption and iOS Keychain. See the [task log](../../../odd/tasks/p10-07-session-store.md).

## Existing state and evidence

ADR-052 leaves session storage separate from HTTP. Identity policy rotates the
access/refresh pair together. Mobile hosts remain integration shells. Native
tools exist locally, but runtime secure-storage tests have not yet been observed.

## In scope

Shared credential/storage contract, redacted errors, a single versioned pair,
native secure persistence, native/common regression tests and integration proof.

## Out of scope

Login/logout/refresh orchestration, repositories, UI, biometric unlock policy,
cross-process access, remote backend calls and mobile CI rollout.

## Constraints and invariants

No plaintext fallback, secret logs or synchronized/backed-up credentials. Native
storage absence must not hide read errors. Backend namespaces remain isolated.
Use synthetic credentials and isolated native test state only.

## Steps

- [ ] Shared contract and deterministic codec tests (observed RED/GREEN).
- [ ] Android Keystore-backed implementation and native tests.
- [ ] iOS Keychain implementation and native tests.
- [ ] Shared tests, artifacts, native checks and independent verification.

## Acceptance criteria

Read, replace and clear a complete pair across fresh store instances. Reject
corruption without returning partial credentials. Native tests verify actual
Keystore/Keychain operations; common/JVM tests alone are insufficient.

## Verification

Commands and observed results will be recorded here. No passes claimed yet.

## Risks, assumptions, and decisions

Risk: items 1, 2, 4 and 5 (credential format/security, ordering and native tests).
Maintainer approved one PR with a size exception. Native review remains disabled
unless the user changes its switch. New shared and native code needs independent
security/failure verification. Native access-control or emulator limitations must
be reported as unavailable proof, never silently bypassed.

## Completion notes

Pending.
