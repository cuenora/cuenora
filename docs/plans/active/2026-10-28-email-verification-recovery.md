# Execution plan: Email verification and password recovery design

- **Status:** Design documented and checked locally; PR merge pending
- **Date:** 2026-10-28
- **Roadmap task ID(s):** P04.12
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Complete “Design email verification and password recovery if passwords are owned.”
Document future protocols and acceptance cases, not working endpoints or delivery.

## Existing state and evidence

Baseline `1129e79` has pending registration, login, rotation, bearer identity and
P04.11 revocation. It has no challenge tables, email sender or verification/reset
endpoints. Resend is already the planned provider; no account was accessed.

## In scope

Product protocol, ADR, future storage/transaction boundaries, abuse/delivery
failure handling, acceptance matrix and correction of related status references.

## Out of scope

Runtime code, migrations, OpenAPI changes, dependencies, email/provider setup,
mobile/web clients, deployment, remote publication and financial authorization.

## Constraints and invariants

The maintainer approved session-bound verification and recovery that revokes all
sessions/remaining recovery links, requires normal login and does not verify email.
Preserve existing email comparison, password policy and pending-user login.

## Steps

- [x] Map current contracts and confirm recovery/verification policies.
- [x] Document protocols, tradeoffs and future acceptance cases.
- [x] Validate documentation and record design-only completion.

## Acceptance criteria

- [x] Verification binds token, current email and authenticated account.
- [x] Recovery specifies an atomic credential/session transition without auto-login.
- [x] Expiry, resend, replay, concurrency, privacy and delivery failures are defined.
- [x] Future implementation boundaries and checks are explicit.

## Verification

Repository validator, whitespace checks, local-link checks and structural readback.
RED/GREEN and runtime suites are not applicable: no runnable behavior changes.

## Risks, assumptions, and decisions

Approved policies are distinct from proposed implementation defaults. TTLs,
attempt budgets and bounded delivery defaults are documented with their tradeoffs.
Implementation needs dedicated migrations/tests; this design does not prove them.

## Completion notes

Design and ADR-022 documented; related identity-storage text now correctly links
the already implemented P04.11 logout routes. Existing OpenAPI, code, migrations,
configuration and dependencies are unchanged. P04.12 is checked for **design only**.

Observed: repository validation passed; git diff --check passed; 194 local Markdown
link targets checked across nine documentation files; structural readback confirmed approved policies, protocol
states and future acceptance cases. No runtime suites run or claimed.
Keep this record active until verified merge. Publication is tracked by the PR;
local documentation checks do not prove the future runtime implementation.
