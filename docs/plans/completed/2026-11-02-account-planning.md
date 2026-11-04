# Execution plan: corrected balance history and account planning

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap task ID(s):** P09.05
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/102
- **Owner:** Agent

## Goal and user-visible outcome

Approved daily `GET /v1/accounts/{id}/balance-history?date_from=…&date_to=…`
(maximum 366 inclusive days), using the currently corrected business-date ledger.
Approved `GET /v1/reports/account-planning?through=…`, default profile-zone month end:
current balance minus separate pending/projections per account; unassigned visible;
incomplete inputs never yield a definitive affected account difference.

## Existing state and evidence

Existing owner-scoped balance reads sum append-only movements; corrections reverse
original business dates and append replacements. Domain `next_occurrence` preserves
calendar anchors. Generation cursor and multirange exclusions retain pause/edit
coverage; the generator writes only through rule-local today. P09.01 establishes
projection precedence, overdue inclusion and current snapshot planning semantics.

## In scope

- Daily carry-forward balance history, exact sums, owned active/archived accounts.
- Read-only future recurrence projections, pending saved terms/effective due dates,
  incomplete due-slot coverage, account/unassigned currency totals and cutoff default.
- Public PostgreSQL tests, synchronized API/docs and independent verification.

## Out of scope

Historical as-known audit reconstruction, actual generation/posting, migrations,
dependencies, UI, implicit FX, indexes, publication and other roadmap completion.

## Constraints and invariants

Reuse `BusinessDate`, `next_occurrence`, profile IANA parsing and exact decimal-text
formatter. Parameterized authenticated ownership, repeatable-read read-only snapshots,
2-second lock and 5-second SQL statement limits. No clock supplied by clients.
No binary floating point or aggregate i64 narrowing. Preserve P09.01–P09.04/.atl.
Risk: items 2 and 3 (financial ownership and shared router/formatter).

## Steps

- [x] Public-interface RED for both missing routes.
- [x] Daily ledger query, then read-only planning with bounded calendar enumeration.
- [x] HTTP GREEN, recurrence/report regression suites, workspace and API checks.
- [x] Independent correction/ownership/no-write/projection verification.

## Acceptance criteria

History carries prior movements, uses original-date reversals plus replacements,
zero-fills days and preserves exact currency sums; 366 accepted/367 rejected.
Planning counts overdue saved planned terms through cutoff, excludes confirmed/skipped,
deduplicates persisted future slots, retains postponed/paused/end/archived references,
projects only rule-local future eligible slots, shows unassigned per currency, and
suppresses affected differences on missing due-generation coverage. Reads never write.

## Verification

| Check | Observed result |
| --- | --- |
| Absent-route RED | Three public HTTP groups failed because missing report routes lacked authenticated no-store responses. |
| Initial HTTP GREEN | Three groups passed. |
| Pre-pause coverage RED | New fourth group demonstrated that active-only rule loading incorrectly marked pre-pause missing backlog complete. |
| Pre-pause correction | Include paused rules only through min(today, day before pause); honor exclusion/cursor coverage. |
| Transient compile failure | Rule query expanded from six to eight fields before its tuple alias; Clippy/harness compilation failed. Alias corrected; all checks below passed afterward. |
| Disposable `test_account_planning_against_disposable_postgres` | Four own groups passed; seven final groups passed after promoting three independent probes and adding foreign planning isolation (18.880s final harness). |
| Disposable flow_reports/account_listing/recurring_generation/recurring_rules | All passed on corrected candidate. These are four sibling harnesses in addition to the account-report harness. |
| `cargo fmt --all -- --check` | Passed after normalization. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed, repeated after final probe promotion. |
| `cargo test --workspace --all-features --locked` | Passed before promoting additional probes; DB groups ignored there and separately exercised above. |
| `python3 tests/infrastructure/test_openapi.py -v` | All seven structural guards passed; not full external OpenAPI validation. |
| `python3 scripts/check_repository.py`; `git diff --check` | Passed. |
| Independent fresh-scratch PostgreSQL verification | Seven groups passed; no blockers observed. |

Independent probes cover public linked-expense correction December→January with
account reassignment, original-date compensation/carry-forward and void; exact
history/current/difference sums exceeding i64; all four cadences, inclusive ends,
month-end/leap/2100 adjustment and native exclusions; archived assignments, persisted
slot precedence and independent unassigned currencies. Opposite rule-local dates
classify one slot as due backlog versus future projection. All public-table hashes
remain unchanged across accepted/rejected/locked reads; exclusive movement-table
lock yields safe 503 for both endpoints, without partial report fields.

No existing parser/formatter logic changed. Sibling public regression suites passed;
no CLI command changed, so CLI history-differential probes are not applicable.
Independent coverage is not exhaustive over every edit/multirange/state/currency
combination; this is a coverage limitation, not an observed defect.

## Risks, assumptions, and decisions

All endpoints use current consistent snapshots, not historical pending states.
Planning bounds: 1,000 accounts, 1,000 recurring rules and 10,000 candidate slots per
request; excessive work rejects safely with 422 planning_too_complex, never partial
success. Pending aggregation stays set-based. Resource safety limits are not financial
cutoff defaults. Completed pause and generation/edit coverage remain authoritative.

## Completion notes

Implemented `account_reports.rs` (transport, authorization/read snapshot and daily
movement sums) and `account_planning.rs` (pure calendar enumeration, native SQL
eligibility/slot precedence and exact set-based totals). Reused existing BusinessDate,
next_occurrence, trusted local-day/profile-zone helpers and decimal-text formatter.
Added API/error/router wiring, OpenAPI/schema guard, permanent PostgreSQL regressions,
product/setup docs and master-plan local completion. Promoted independent probes,
normalized, reran focused tests and removed outside scratch after parent readback.

No new dependencies, migrations, financial writes, indexes, CLI or UI. Planning
resource bounds fail closed with 422; query profiling/performance remains P09.10.
Prior P09.01–P09.04 and .atl/ preserved. RDD globally off; no native review lifecycle,
commit, remote operation or publication. Keep this record in active/ until merged.
Next: P09.06. Risk: items 2 and 3, independently verified.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `69d1150 feat(reports): implement dashboard metrics and reporting (P09.01-P09.05) (#102)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
