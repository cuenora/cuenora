# Execution plan: Effective dates and time zones

- **Status:** Completed — PR #26 merged and all four hosted checks passed
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P03.03
- **Tracking issue / PR:** [PR #26](https://github.com/cuenora/cuenora/pull/26)
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Define financial business dates, audit instants and named time zones so a trip,
backdated entry or daylight-saving transition cannot silently shift records.
P03.03 is a specification task; it does not implement date types or persistence.

## Existing state and evidence

- P03.01–P03.02 are merged in PR #24. Their completed plan was archived in PR #25;
  both PRs have four successful hosted checks. Local main is `9e8071a`.
- The roadmap already separates `occurred_on DATE` from UTC audit timestamps
  and requires an IANA user time zone. Domain code currently implements money only.
- The maintainer explicitly approved each of four date policies: a stable profile
  zone, no future-dated realized entries, confirmed initial device-zone suggestion,
  and recurrence zones unaffected by a profile-zone change.

## In scope

- A date/time specification, ADR and deterministic acceptance examples.
- Business-date formats, bounds, today evaluation and audit-time precision.
- References and the P03.03 roadmap checkbox after structural verification.

## Out of scope

Runtime code, libraries, migrations, profile UI, scheduling engines, month-end
recurrence rules, interest calculations, metric formulas and additional bank dates.

## Constraints and invariants

Keep business dates independent of UTC instants; do not shift accepted history.
Validate on the server with an injected instant and stored named zone. No client
audit clock or server/device default zone may silently decide financial dates.
Preserve `.atl/` and the existing money implementation.

## Steps

- [x] 1. Inspect the roadmap and confirm unresolved product decisions one at a time.
- [x] 2. Specify formats, date boundaries, zone changes and audit semantics.
- [x] 3. Verify examples independently and update documentation references.

## Acceptance criteria

- [x] Approved policies are distinguishable from engineering decisions and deferred rules.
- [x] Examples cover midnight, leap dates, future rejection, travel and DST transitions.
- [x] No migration, date library or financial API is implied to exist.
- [x] P03.03 is checked only after the definition and its examples are verified.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Python `datetime` / `zoneinfo` fixed-input examples | Dates and UTC projections match the specification | Four valid dates, nine invalid dates, five Stockholm projections, three date comparisons, New York projection and microsecond flooring passed | Passed |
| Relative Markdown link and structural readback | All changed local targets exist | All links and table structures valid across seven documents | Passed |
| `python3 scripts/check_repository.py` | Repository text validation passes | Passed; 99 Git-visible files examined | Passed |
| `git diff --check` | No whitespace errors | Passed | Passed |
| Rust RED/GREEN | Not applicable to a passive specification | No runtime source changes | Not applicable |

## Risks, assumptions, and decisions

See [date semantics](../../product/dates-and-time-zones.md) and
[ADR-003](../../architecture/decisions/003-date-semantics.md). Initial supported
date bounds and audit precision are engineering choices, not bank-wide policies.
Revisit before extending the interchange range or adding local-time scheduling.

## Completion notes

Work is inline on `docs/date-and-identity-semantics`, based on merged `main` at `9e8071a`.
PR publication was requested on 2026-10-24. Archived after verified merge.
Risk: none; documentation only.

The 17 acceptance groups are specifications for future value-type, use-case and
persistence tests. Independent Python checks verify their calendar arithmetic;
they do not claim that future workflow effects have been implemented or tested.
No Rust formatter, Clippy, runtime tests or PostgreSQL tests were required for
this passive definition. No failures remain open.

Published in [PR #26](https://github.com/cuenora/cuenora/pull/26) against `main`.
The maintainer explicitly accepted `size:exception` on 2026-10-24 for the combined
date/identity definition (initial commit `f5237b8`, 537 changed lines).
Runtime implementation and merge are not included in this publication task.

PR #26 was merged by the maintainer on 2026-10-24 at 10:56:16 UTC as
`78356897c1536bbcdf063c135899a3ff6b538152`. Repository / Validate,
Backend / Rust, Backend / Audit and Backend / Dependency Review all passed.
Local `main` was fast-forwarded; its tree matches published head `ca16c76`.
This closes the P03.03 definition only; runtime date behavior remains pending.
