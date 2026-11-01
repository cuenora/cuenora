# Execution plan: Savings configuration

- **Status:** Completed locally; publication and merge pending
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P08.03
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Verified owners can append and query independent savings terms and rate histories.
See [task specification](../../../odd/tasks/p08-03-savings-configuration.md).

## Existing state and evidence

Migration 0021 already enforces savings-only immutable effective-dated history.
No configuration endpoint exists. RDD is globally off.

## In scope

- Authenticated verified-owner configuration, bounded ordered history reads, exact percentage strings.
- Public HTTP regression tests, OpenAPI and setup documentation.

## Out of scope

Corrections, deletion, calculations, forecasts, actual interest, mobile and remote operations.

## Constraints and invariants

No financial effects. Preserve independent streams and require explicit policies.
Reject foreign/normal accounts without disclosure. Existing history cannot be overwritten.

## Steps

- [x] Implement and test append/query operations.
- [x] Synchronize contract and documentation.
- [x] Run own checks and independent ownership verification.

## Acceptance criteria

- [x] Histories retain historical/future dates and exact nonnegative rates with <=6 fractional percentage digits.
- [x] Invalid, conflicting and unauthorized requests change no configuration or financial records.
- [x] Identical retries return the existing version; changed same-date payloads conflict.
- [x] Reads are owner-scoped and bounded; no calculator or financial posting is introduced.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Disposable savings configuration HTTP target | All acceptance groups pass | 3 groups passed; final parent rerun passed | Passed |
| Cargo format, Clippy, workspace tests | Pass | Formatting/Clippy passed; independent workspace suite 111 passed, 184 ignored (includes one scratch-only differential test) | Passed |
| Repository checker | Pass | Passed | Passed |
| Independent security verification | No blockers | Ownership, expiry/revocation, concurrent rate conflict, unchanged rejected snapshots passed; S1/S2 pass after contract correction | Passed |
| Existing account listing HTTP regression | Pass | All 9 disposable PostgreSQL groups passed | Passed |
| Contract structure and parsed refs | Pass | 2 structural tests passed; 371 YAML local refs resolve | Passed |

## Risks, assumptions, and decisions

Risk: item 2 (ownership and verified-email authorization), item 4 (retry serialization).
Terms and rates use separate dated resource slots. Identical retries are naturally
idempotent at the account/date slot; a different payload conflicts, never rewrites.
Explicit financial dates need no inferred today or profile-zone conversion.
Read history uses effective_on cursor and limit (default 50, maximum 100).

## Completion notes

Implemented two independent GET/POST history resources under owned accounts.
Same stream/account/date is the permanent retry identity; equivalent retries return
the original version and differing payloads conflict. All policies are explicit.
No migration, dependency, calculator or financial posting change.

The public-interface tests were observed RED before routes existed, then GREEN.
An intermediate unsupported-method response lacked a typed JSON body; route-local
method fallback corrected it. Independent verification found malformed OpenAPI
response placement caused by an unanchored text replacement; it was corrected in
one batch and pinned with standard-library structural tests (RED then GREEN).
The verifier's initial suite had a missing-rlib doctest failure caused by concurrent
baseline/candidate builds sharing a target directory; its sequential rerun passed.
Ignored Cargo database tests are not counted as PostgreSQL proof.

Independent baseline/candidate differential checks preserved normalized status/body
for existing account GET/POST, categories and unknown routes. Concurrent conflicting
rate writes and expired/revoked credentials were independently probed and incorporated
into committed regressions. The final parent focused PostgreSQL rerun passed.

Maintainer approved one PR with a size exception. RDD remained globally off.
No push, PR creation or merge is authorized or performed. Keep this record under
active until merge. The pre-existing P08.02 roadmap text still says uncommitted
although its storage is present in baseline main; this unrelated status wording
was not silently rewritten.

Risk: item 2 (verified-owner authorization), item 4 (concurrent date-slot retries).
Rollback boundary: remove only savings route wiring, the two savings modules,
their new error variants, tests and associated contract/docs; migration 0021 is
unchanged and existing configuration rows remain intact.
