# Execution plan: Exact shared amount serialization

- **Status:** Completed locally; unmerged
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.05
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Serialize financial amounts as exact JSON strings in a transport adapter, without
adding transport dependencies or annotations to domain models. The maintainer
approved JSON integration with kotlinx.serialization, not only a text codec.

## Existing state and evidence

ADR-050 keeps domain models pure. Money stores symmetric Long hundredths;
ProjectedMinorUnits stores arbitrary-precision nonnegative integer digits.
OpenAPI ReportAmount and projected_payout use two-decimal strings. Rust Money
accepts integer or one/two fractional digits and emits exactly two digits.

## In scope

- Currency-bound Money serializer with exact parsing and canonical output.
- ProjectedMinorUnits serializer and validated arbitrary-precision report strings.
- kotlinx.serialization JSON 1.11.0 runtime, custom serializers without compiler plugin.
- Shared JSON tests, native artifact builds, independent dependency verification.

## Out of scope

Full resource DTOs, HTTP/client errors (P10.06), rates, rational arithmetic,
cross-language fixture suite (P10.11), and financial posting or authorization.

## Constraints and invariants

No floating point, rounding, locale parsing, domain mutation or implicit currency
conversion. Reject numeric JSON tokens even with lenient JSON settings. Currency
comes from the containing resource, not the amount string. Nullable projections
use the standard nullable serializer. Report totals must never narrow to Long.

## Steps

- [x] 1. Observe failing shared JSON behavior tests.
- [x] 2. Implement adapters and document the transport boundary.
- [x] 3. Run both targets, allTests, artifact builds and repository checks.
- [x] 4. Independently verify dependency integration and edge cases.

## Acceptance criteria

- [x] Strings preserve exact values, including symmetric bounds and huge projections.
- [x] Malformed, overprecision, overflow and non-string inputs fail without rounding.
- [x] Existing domain and marker tests remain green on both execution targets.
- [x] Native artifacts build; no domain annotations or JSON dependency imports.

## Verification

From mobile with documented JBR/Android SDK environment, run
`:shared:testAndroidHostTest :shared:iosSimulatorArm64Test`, then `:shared:allTests`
and the three documented shared artifact tasks. Run repository checker at root.
Observed results will be recorded below.

## Risks, assumptions, and decisions

Risk: item 5 (new shared runtime dependency affects Android and Kotlin/Native).
Custom serializers need no compiler plugin. JSON strings are field values, not
whole resource objects; resource DTO mapping stays with the future client.
Kotlin/Native Xcode compatibility and debug-cache warnings may persist.

## Completion notes

Observed RED: Android-host tests ran against empty serializers; three of five
cases failed at conversion assertions. GREEN: all 14 tests per Android-host and
iOS-simulator target passed, zero failures/errors/skips. allTests, all three shared
artifact tasks, repository validation and git diff --check passed.

Independent verification used isolated baseline/candidate snapshots: nine baseline
tests per target, 14 candidate tests, 15 with scratch-only 10,000-digit/boundary
probes, all passing. All candidate artifacts built. Existing domain sources/tests
matched baseline byte-for-byte. No change-caused blockers. Xcode/debug-cache
warnings also occurred on baseline. Parent removed verification scratch.
Native review is disabled globally and was not started.

Added adapters, tests, runtime dependency and ADR-051; updated mobile docs and
P10.05 only. No domain, HTTP or storage changes. No commit or publishing requested.
P10.06 remains the next client task.
