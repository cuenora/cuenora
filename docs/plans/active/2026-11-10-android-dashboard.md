# Execution plan: Android dashboard and statistics

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-10
- **Roadmap task ID(s):** P11.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Home compares each account's current recorded balance with pending and projected
payments. Insights shows realized flows, monthly trends and spending categories
for an independently selected inclusive period, grouped by currency.

## Existing state and evidence

The planning, flows, monthly-flows and expense-distribution APIs exist. Home and
Insights remain placeholders. See [the task log](../../../odd/tasks/android-dashboard.md)
for exact authorization, unit boundaries and verification evidence.

## In scope

- Existing read-only reports with exact amounts and completeness.
- Home and Insights, navigation and refresh after existing financial writes.
- Shared/native tests, build/lint and independent boundary verification.

## Out of scope

Backend/schema changes, financial writes, live-bank claims, implicit FX totals,
remote operations, physical-device access and iOS product UI.

## Constraints and invariants

Follow [dashboard metrics](../../product/dashboard-metrics.md) and root AGENTS.md.
Incomplete reads are not zeros. Savings is a balance subset, not extra money.
Authentication, ownership and retired-screen isolation remain mandatory.

## Steps

- [x] Home planning integration and tests.
- [x] Insights integration and tests.
- [x] Authenticated wiring, documentation and final verification.

## Acceptance criteria

- [x] Per-account differences, unassigned/archived visibility and currency separation.
- [x] Independent explicit dates and profile-zone initial reporting month.
- [x] Failure/loading/empty/incomplete states without fabricated values.
- [x] Owner, retirement and navigation regression checks.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared focused then full Android-host/iOS-simulator suites | Exact reports and protected lifetime | Final Android-host231 / iOS-simulator232; zero failures/errors/skips | Passed |
| Isolated API37 native suite | Screen/navigation compatibility | Full117:113 passed /4 new-test failures; corrected assertions/scroll recheck6/6; shared-device7/7 | Passed with bounded recheck |
| APK, AAR and two iOS frameworks; standalone lint | Build and static checks | APK/test APK, AAR, two frameworks and standalone lint (zero issues) | Passed |
| Repository check and diff whitespace | Consistent repository | Repository validation and diff whitespace | Passed |
| Independent baseline/candidate probes | No change-caused regression | Baseline/candidate194 legacy cases identical; five independent probes and original-blocker recheck passed | Passed |

## Risks, assumptions, and decisions

Maintainer approved both screens, initial Insights current month in the confirmed
profile zone, and one future size-exception PR. Local work units stay separate.
Risk: items 2/3/4 (identity, shared navigation and suspended-response lifetime).
Native review is globally disabled; no mode change or native review will run.

## Completion notes

Implemented Home planning and Insights with existing read-only contracts; no backend,
schema, dependency or financial-write changes. The task log records local commits.

### Verification evidence

- Focused reports32 cases passed. Initial Insights compile-RED and correction RED
  were observed. T1 source preceded initial tests: this process deviation was
  disclosed; later correction tests had observed RED and GREEN, not retroactive proof.
- Independent probes found one late Planning profile401 failure that bypassed
  disposal. Moving the guard before both result branches fixed it; the exact
  original blocker passed its independent recheck. The five probes are retained
  as shared regression tests. Baseline194 and candidate194 legacy case identities,
  outcomes and captured outputs matched; existing shared production is unchanged.
- Final shared suites:231 Android-host and232 iOS-simulator, zero failures/errors/
  skips. AAR, iOS device/simulator frameworks, debug APK/test APK and standalone
  lint passed. Existing Kotlin/Native keychain/linker-cache and Compose-test
  deprecation warnings remain; no new lint issues.
- Full owned API37 native run:117 app cases,113 passed and4 failed. The new tests
  used obsolete/exact notice text without the existing `Error ·` prefix and tried
  clicking an offscreen category row. Assertions were corrected to current exact
  copy, and the row is scrolled into view before clicking. The bounded Home/Insights
  recheck passed6/6; shared-device storage passed7/7 separately. No product
  assertions were removed. This is not a claim of a second clean full117 run.
- Native navigation regression tests passed in the full run. The emulator initially
  vanished between tool calls: a controlled background `sleep` experiment confirmed
  executor child cleanup when the launching shell exits. Keeping launch, boot and
  tests in one foreground shell preserved the owned emulator. No physical device,
  existing emulator, remote API or real financial data was used.

### Scope and remaining limitations

Home shows current snapshot balances and commitments through its selected cutoff,
not historical rewinding or guaranteed spendable money. Insights has three separate
report reads; its category total is preserved independently. History drilldowns
explicitly disclose unsupported currency/kind/uncategorized filters rather than
claiming exact contributions. Existing in-app financial flows refresh loaded views.

Native RDD stayed globally disabled (disabled/unmanaged); high-risk assessment
was informational and did not enable review. No hosted CI, physical-device,
minimum-API runtime, full accessibility or live-backend proof is claimed.
No push, PR, merge or deployment was authorized or performed.
