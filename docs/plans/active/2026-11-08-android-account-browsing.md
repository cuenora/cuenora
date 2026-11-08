# Execution plan: Android account list and details

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-08
- **Roadmap task ID(s):** P11.04
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Browse paginated accounts and inspect a selected account's metadata and exact posted balance in native Android screens. Activity remains P11.06, explicitly confirmed by the maintainer.

## Existing state and evidence

- Main b47a496 contains Android authentication and first-account onboarding.
- The Accounts destination is a foundation placeholder.
- Shared AccountRepository.list returns metadata, currency-matched Money and a cursor; no dedicated account detail GET exists.
- AuthSession.authorized serializes resource calls, credential renewal and logout.
- Detailed specification and running evidence: [feature logbook](../../../odd/tasks/android-account-browsing.md).

## In scope

- Authenticated account reads, paginated list and selected-record detail.
- Loading, empty, errors, explicit read retries and exact amount/currency display.
- Native navigation/Back, preserved list context and session isolation.
- Shared adapter tests, Android state/Compose tests, isolated independent probes.

## Out of scope

- Transactions/activity, pending payments, savings projections and totals.
- Rename/archive/reactivate, additional account creation and financial writes.
- New backend endpoints, schemas, dependencies, iOS screens or deployment.

## Constraints and invariants

Use existing theme/components and shared HTTP/session adapters. Never use floating point for money, manufacture a zero on read failure, persist financial presentation in saved state, cache a bearer, or infer ownership from login input. Preserve P11.03 onboarding and original auth behavior. Refresh is read-only and explicit; a selected balance is the result of its last successful read, not live bank data.

## Steps

- [x] 1. Test and implement authenticated account reads.
- [x] 2. Test and implement Android list/detail integration.
- [x] 3. Run own checks and independent security/concurrency verification.
- [x] 4. Update delivery evidence and master-plan status only with observed proof.

## Acceptance criteria

- [x] Exact signed balances, currency, normal/savings and active/archived metadata are visible.
- [x] Pagination and retry do not duplicate rows or lose existing list context.
- [x] Read failure is distinguishable from confirmed empty data.
- [x] Detail/Back preserves the originating account list; session loss removes sensitive screens.
- [x] Existing onboarding and foundation navigation remain functional.
- [x] Applicable tests and independent security/order checks have observed results.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared focused adapter tests, then Android-host/iOS shared suites | Correct authorized requests and existing behavior | Reader6 and full 115/116 passed; allTests and three shared artifacts passed | Passed |
| Android focused tests, then isolated API37 instrumentation suite | List/detail, pagination, recovery and existing UI pass | Focused 16 and full 35 passed, no failures/errors/skips | Passed |
| Android assembleDebug and lintDebug separately from shared tests | Build passes; baseline warnings identified | Passed; four baseline lint warnings remain | Passed |
| python3 scripts/check_repository.py | Repository policy passes | Passed; final closure readback below | Passed |
| Independent security/order probes in external scratch | No candidate-caused blockers | PASS S1/S2, six reader and twenty native cases; scoped baseline/helper recheck passed | Passed |

## Risks, assumptions, and decisions

Risk: items 2 and 4 — authenticated sensitive data and async ordering. Backend remains authoritative for ownership and verified email. Metadata/balance detail reuses list records rather than introducing a new API. RDD disabled/unmanaged. Physical devices, API23 runtime, live backend and full assistive-technology certification are not implied by isolated tests.

## Completion notes

Implemented through the existing authorized profile/account adapters and native
Compose components. Final tests: 115 Android-host, 116 iOS-simulator and 35 API37
instrumented, zero failures/errors/skips. Shared AAR/two frameworks, allTests,
app debug/test APKs and lint passed. Parent reader 6/build/lint spot checks passed.

Independent verification found no candidate-caused blocker. Original auth automation
failed identically on baseline and candidate because the submit button was IME-clipped
and repository calls remained zero. A test-only keyboard dismissal/scroll helper keeps
all original assertions; independent baseline 6 and final candidate 35 now pass. The
four new independent regression probes are retained, including the complete original
credential-clear trace. Runtime auth source was not changed.

Existing warnings: four lint warnings (old target, navigation update, backup extraction,
launcher icon), Kotlin/Native cache and Keychain pointer/redundant cast warnings. No
API23 or full accessibility proof is claimed; the later authorized physical/hosted
manual smoke is recorded below. No backend changes,
PostgreSQL tests or Rust suite are applicable because backend bytes are unchanged.

Single future PR size exception approved with separate local behavior commits. No
issue, PR, push or merge authorized. The later backend/device authorization is
limited to the manual read-only smoke below. Keep this execution record in
active/ until integration. RDD remained disabled/unmanaged. Commit identities and
cleanup evidence are recorded in the linked feature logbook.


### Authorized physical-device smoke — 2026-11-08

The maintainer authorized the connected Samsung SM-S908B and the configured
`https://cuenora.carf.dev` backend for account reads, with credentials entered
only by the maintainer. ADB confirmed API36. Update installation initially failed
with `INSTALL_FAILED_UPDATE_INCOMPATIBLE`; public certificate comparison showed
the local standard debug key did not match the installed app. The maintainer
explicitly authorized uninstall/reinstall with local data loss. Uninstall, fresh
installation and Activity launch succeeded; device commands changed no server
financial records. No credentials, financial data, screenshots or UI hierarchy
were captured. Temporary certificate-comparison files were removed.

The maintainer confirmed each UI outcome separately:

| Manual scenario | Maintainer-reported result |
| --- | --- |
| Open Accounts after sign-in | Accounts show name, type, status, currency and balance |
| Open account details | Metadata and balance match the listing |
| Native Android Back | Original accounts and list position retained |
| Refresh accounts online | Loading finishes with correct records, no errors/duplicates |
| Refresh accounts without connectivity | Explicit error and Try again, not fabricated empty records or zero balances |
| Reconnect and Try again | Correct accounts and balances return |

This is a successful scoped manual smoke, not automated hosted E2E or captured
network/server telemetry. It does not prove device pagination beyond the visible
records, another Android version, session renewal, full accessibility, or financial
writes. Activity remains P11.06; no backend administration or publication occurred.
