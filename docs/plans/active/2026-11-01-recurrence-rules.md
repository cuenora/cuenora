# Execution plan: Recurrence rule lifecycle

- **Status:** Locally completed; merge pending
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P07.07
- **Tracking issue / PR:** Not created; local delivery only
- **Owner:** Agent with maintainer decisions

## Goal and user-visible outcome

Verified owners can create expense rules, edit their terms and calendar, pause
and resume them without posting money or rewriting existing occurrences.

## Existing state and evidence

Migration 0017 stores rules, immutable occurrence slots, inclusive generation
progress and completed pause exclusions. The pure calendar calculator exists.
The API has reusable bearer middleware, exact money/date validation, named-zone
parsing, owner-first financial serialization and disposable PostgreSQL tests.
No recurrence API currently exists. See `odd/tasks/p07-07-recurrence-rules.md`.

## In scope

- POST `/v1/recurring-rules` with permanent owner-scoped creation idempotency.
- PATCH `/v1/recurring-rules/{id}` with absent-versus-null partial updates.
- POST `/v1/recurring-rules/{id}/pause` and `/resume`, accepting `{}` only.
- Exact expense terms, ownership/verification, compatible accounts/categories,
  explicit named rule zone, inclusive end, state-preserving edits and exclusions.
- An additive retry-key migration, HTTP contract and executable regressions.

## Out of scope

Generation, workers, occurrence actions, financial posting, deletion and listing.
No dependencies, remote operations or production data. RDD stays disabled.

## Constraints and invariants

Reuse existing money, date, UUID and zone parsers. Serialize owner → rule → account
in READ COMMITTED transactions with bounded lock/statement timeouts. Never trust
client ownership, state, progress or audit metadata. Preserve occurrences,
transactions, movements and financial audit. Rules may be accountless or use
Normal/Savings accounts; planning is not posting. New or changed account selection
must be active; unchanged references survive archival.

## Steps

- [x] 1. Observe RED for public HTTP rule creation, edits, pause/resume and guards.
- [x] 2. Implement atomic rule lifecycle, retry storage and synchronized contract.
- [x] 3. Observe GREEN, run workspace/repository checks and independent verification.

## Acceptance criteria

- [x] Exact positive SEK/EUR/USD expense terms, compatible owner account/category,
      four supported cadences, explicit IANA zone and inclusive valid end.
- [x] Permanent creation retries return the original creation representation even
      after edits/pause; conflicting keys reject without mutation.
- [x] Partial edits distinguish absence from explicit null and preserve state.
- [x] Actual start/cadence/zone changes advance coverage past the edit day and prior
      cursor; terms-only and no-op edits do not suppress active backlog.
- [x] Pause/resume use one server instant in the rule zone; repeated assignments
      are no-ops, and resume unions `[paused_on,today)` without resetting the anchor.
- [x] Rejections are atomic; no foreign rule/account or unverified access succeeds.
- [x] Existing occurrence/financial snapshots remain byte-equivalent.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Disposable `recurring_rules` HTTP target | All lifecycle/guard cases pass | 9/9 passed; migration/history/key constraints passed | Passed |
| Rust fmt, Clippy, workspace tests | Pass; DB cases explicitly ignored outside harness | All passed on pin 1.99.1; DB cases correctly ignored | Passed |
| `python3 scripts/check_repository.py` | Pass | Passed, 366 visible files | Passed |
| Independent verifier (risk 1/2/4) | No change-caused blocker | S1–S4 passed; 4 independent DB groups, 12 edit/pause races, 16 identical baseline/candidate HTTP results | Passed |

## Risks, assumptions, and decisions

- Risk items 1, 2, 4: persisted lifecycle/exclusions, ownership, serialized writes.
- Maintainer approved future-only start/cadence/zone edits, preserving occurrences.
  Use the maximum of old/new-zone edit days and existing covered progress so a
  zone change cannot expose an already-covered civil date.
- Maintainer approved rejecting archived accounts on create/reassignment while
  preserving unchanged references. Foreign accounts return opaque 404.
- API/retry choice follows existing financial routes: mandatory bounded ASCII
  Idempotency-Key on creation, permanent hashes, replay of initial representation.
  Mutation retries assign the current state; a delayed retry after the inverse
  action is a new assignment, not a historical operation replay.
- Pause/resume dates are server-derived, never caller-provided. A backwards civil
  clock/zone transition that would close a pause before its start rejects atomically.
  Zone edits preserve civil-date exclusion labels and open pause date.
- No profile zone is required: every dated transition uses the explicit rule-owned
  zone rather than the profile zone, and scheduled dates may be future dates.
- One cohesive size-exception delivery approved; no publication authorized.

## Completion notes

Implemented locally; full disposable PostgreSQL suite passed all 69 groups in
1421.408 seconds. OpenAPI unique-key parse and internal-reference validation passed.
Independent verification passed S1–S4 without blockers. The sole advisory was a
pre-existing stale lifecycle status sentence, now corrected. Parent spot-check
of the independent database command passed (82.844s); details are recorded in the
task log. The active plan remains here until merge.
