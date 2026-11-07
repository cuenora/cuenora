# Execution plan: Shared form validation

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.09
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agents

## Goal and user-visible outcome

Provide UI-independent local validation for account, transaction, authentication, recurrence and savings configuration forms. See the [task specification](../../../odd/tasks/p10-09-form-validation.md) for exact authorized scope and verification evidence.

## Existing state and evidence

Shared domain constructors and exact JSON amount adapters exist; a form layer does not. Backend registration uses strict IDNA and a common-password list; profile zones use chrono-tz. These server checks are not available in common Kotlin.

## In scope

Typed field/cross-field local validation and common tests; documentation of server-only checks.

## Out of scope

UI/presentation state, new dependencies, server changes, auth orchestration, schedule generation and financial calculations.

## Constraints and invariants

Exact decimal input, existing domain rules, password preservation and no credentials/raw form input in errors. Local acceptance is not authorization or a server acceptance guarantee.

## Steps

- [x] Authentication validators and tests.
- [x] Financial validators and tests.
- [x] Integration, documentation and independent security verification.

## Acceptance criteria

- [x] All five form families are covered by local deterministic checks.
- [x] Android-host and iOS-simulator tests pass; shared artifacts build.
- [x] Server-only checks and edit-form semantics are documented.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared target tests and allTests | Pass on both targets | 73 Android-host / 74 iOS-simulator; zero failures/errors/skips | Passed |
| AAR and two frameworks | Build | All three built | Passed |
| Repository checks | Pass | Python and diff checks passed | Passed |
| Independent security verification | No change-caused blocker | Baseline 58 tests, candidate 73 + 2 probes; prior outputs unchanged | Passed |

## Risks, assumptions, and decisions

Authentication form validation touches credential policy (risk item 2), but never authenticates. User accepted server-side IDNA, common-password and zone catalog checks. Validate complete merged edit forms, not standalone PATCH fragments. Maintainer approved a single future PR size exception on 2026-11-07; use separate local auth and financial work-unit commits. No chain or publication authorized.

## Completion notes

Implementation verified locally; single future PR size exception approved; local work-unit commits recorded below. No publication authorized. Existing Native cache-object and Keychain cast warnings remain. Independent probes found no blockers; quote numeric overflow remains server-side. See ADR-055. No UI/live backend proof.

## Local delivery evidence

- `451a27d`: authentication form validation, result types, tests and ADR-055 (304 lines; risk item 2).
- `ed78a99`: financial form validation, tests and usage documentation (529 lines; risk none).
- Closing documentation commit: `docs(shared): close P10.09 validation task`.
- One future PR with explicit size exception approved. Native review disabled globally; no push, PR creation or merge authorized. Keep this execution record in active until merge.
- Verification command: `cd mobile && ./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test :shared:allTests :shared:assembleAndroidMain :shared:linkDebugFrameworkIosArm64 :shared:linkDebugFrameworkIosSimulatorArm64` with documented JBR/SDK environment. All passed. Parent repository/diff spot checks also passed.
- Independent scratch remains at `/tmp/cuenora-verify-forms.CYejJa`; cleanup was denied by the runtime. It is outside the worktree.
