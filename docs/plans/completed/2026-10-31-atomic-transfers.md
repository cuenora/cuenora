# Execution plan: Atomic transfer creation and permanent retries

- **Status:** Completed
- **Date:** 2026-10-31
- **Roadmap task ID(s):** P06.03, P06.07; P06.02 GREEN prerequisite
- **Tracking issue / PR:** [PR #85](https://github.com/cuenora/cuenora/pull/85) — merged
- **Owner:** Maintainer and implementation agent
- **Merge evidence:** `0638ee17f62f22ed06645023e53be5e0dca6967d` on 2026-10-31; all four hosted checks succeeded.
- **Archived:** 2026-10-31; scoped acceptance criteria verified complete.

## Goal and user-visible outcome

Record a transfer through the existing transaction POST as one SQL transaction,
including two actual-amount movements, minimal audit and permanent retry record.

## Existing state and evidence

P06.01 specifies amounts and rates. P06.02 has nine public HTTP tests observed
RED (`400` rather than `201`). Existing storage supports both transfer amounts;
there is no creation use case or transfer retry table. Income/expense writes use
owner-first locking and independent immutable permanent retry namespaces.

## In scope

- Implement no-quote transfer creation and P06.07 retry equivalence.
- Preserve verified ownership, active/currency-matched distinct accounts and dates.
- Add strict public request/response documentation and acceptance/regression tests.

## Out of scope

- Quoted-rate runtime/storage (P06.06), reporting and mobile applications.
- Completing all later Phase 06 test matrices or claiming the phase exit.

## Constraints and invariants

Exact amounts; one posted event, debit, credit, audit and retry result commit
atomically. Same-key equal requests replay the original acknowledgement even
after edits/voids; different requests conflict. Reuse current helpers, immutable
movements and owner-first lock ordering. Preserve existing income/expense input
rejection and outputs. Never use a personal or production database.

## Steps

- [x] Implement atomic creation and additive immutable transfer-key migration.
- [x] Observe existing RED to GREEN; add retries/conflicts and compatibility cases.
- [x] Run focused and full checks, differential probes and independent verification.

## Acceptance criteria

- [x] Nine P06.02 cases pass through HTTP without fixture-created movements.
- [x] Transfer retry namespace, canonical equivalence and permanent results pass.
- [x] Rejections/late failures leave full financial state unchanged.
- [x] Existing income/expense behaviors remain compatible.
- [x] OpenAPI and setup document the implemented slice and pending quote support.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Rustfmt, Clippy, workspace Cargo | Pass; DB tests explicitly ignored | Passed: 91 ordinary/doctests; 123 DB cases ignored | Pass |
| Disposable transfer target | All cases pass | Previous nine RED; final sixteen groups GREEN | Pass |
| Disposable income/expense/mutation targets | Pass | Passed in independent full infrastructure suite | Pass |
| Full disposable infrastructure suite | Pass with cleanup | Independent 63 groups passed in 1299.300s; prior fourteen-group transfer target. Final expanded sixteen-group target rerun by parent passed afterward | Pass |
| Independent isolated probes and baseline differentials | Pass S1–S3 | PASS S1–S3; 26 normalized income/expense responses unchanged versus baseline; full financial hashes unchanged after rejection | Pass |
| Repository validation and diff readback | Pass | Passed; OpenAPI YAML and schema references checked | Pass |

## Risks, assumptions, and decisions

Risk: items 1,2,3,4,5 (permanent retry schema, server ownership, shared POST,
concurrent requests and additive migration). Independent verification required.
Maintainer explicitly approved P06.07 with P06.03. No remote operation authorized.
Maintainer explicitly accepted a single-PR size exception for approximately
1,670 authored changed lines, including P06.01/P06.02 prerequisites. No reduction
of tests or formatting to fit a line budget.

## Completion notes

Implementation and verification are complete locally. No production logic changed
after independent verification; its additional probes were retained as sixteen-group
regression coverage. Expanding snapshots to include account metadata required a
fixture-only adjustment: take the rejection snapshot after intentional archival.
The parent reran the final target successfully. The full suite was not repeated
after adding those tests; its production source matches the verified source.

Native RDD is globally disabled and was not reactivated. Independent functional
verification still ran. Local work-unit commit `f8a186f` contains the implementation and prerequisites.
The maintainer subsequently authorized HTTPS branch publication with the current
GitHub CLI session and PR creation against `main`, without merge. The maintainer accepted the single-PR size exception:
the coherent atomic/retry implementation, tests and contract exceeds 400 lines
even if the independent P06.01 documentation is split out.
Rollback boundary: new transfer modules and 0015 table, shared POST branch,
transfer error/schema alternatives and their tests/docs; existing income/expense
financial semantics remain unchanged. On populated databases, do not remove
permanent keys; use reviewed forward corrections instead.

The detailed specification/logbook is
[`odd/tasks/p06-03-atomic-transfers.md`](../../../odd/tasks/p06-03-atomic-transfers.md).

Implementation commit: `f8a186f` (1,673 authored changed lines, size exception
approved). [PR #85](https://github.com/cuenora/cuenora/pull/85) was merged into `main`
by the maintainer. All local criteria and hosted checks passed; the verified merge
is recorded above. The agent did not perform the merge. This plan is now archived.
