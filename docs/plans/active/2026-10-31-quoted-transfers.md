# Execution plan: optional exact transfer quotes

- **Status:** Active — verification complete; approved local delivery in progress
- **Date:** 2026-10-31
- **Roadmap task ID(s):** P06.06
- **Tracking issue / PR:** Not created
- **Owner:** Agent

## Goal and user-visible outcome

Record an optional directional quote alongside confirmed transfer amounts and
return exact, informational discrepancy data without changing the actual postings.

## Existing state and evidence

No-quote transfers, all nine supported currency pairs, ownership guards and
permanent retries already exist. The strict adapter rejects `quoted_rate`;
transactions have no quote column. See `odd/tasks/p06-06-quoted-transfers.md`
for the specifications, task routes and live verification log.

## In scope

- Exact positive canonical quotes with at most twelve fractional digits.
- Additive quote storage, optional creation/history fields and retry compatibility.
- Checked half-even indicative amounts and signed differences in destination units.
- Public HTTP, domain and populated-migration regression coverage.

## Out of scope

- Quote editing, mobile UI, automatic exchange, fees and reporting changes.

## Constraints and invariants

Actual amounts remain authoritative. No floating point or new dependency.
Preserve existing no-quote responses, hashes, audit/posting semantics and user edits.

## Steps

- [x] Add regression tests and observe RED.
- [x] Implement domain, persistence, API and documentation together.
- [x] Run own checks and independent high-risk verification.

## Acceptance criteria

- [x] Approved quote grammar, precision and half-even examples pass.
- [x] Valid discrepancies never override postings; invalid requests leave no effects.
- [x] Canonical retries, history, corrections/void preservation and existing behavior pass.
- [x] Migration preserves existing values and rejects invalid stored quotes.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Domain quote tests | Exact parsing and checked half-even results | 2 passed | Passed |
| Disposable transfer and migration targets | Public behavior and atomic storage | 4 targets passed in 93.345s; final expanded 19-group transfer target passed in 15.939s | Passed |
| Workspace formatting, Clippy and tests | No regressions | Rustfmt/Clippy passed; 93 ordinary tests passed, 126 DB tests ignored | Passed |
| Independent isolated verification | S1–S4 satisfied | PASS; 64 infrastructure groups in 1358.740s, independent quote and real migration/replay probes | Passed |

## Risks, assumptions, and decisions

Risk: items 1,3 — financial schema and existing transport/history.
No-quote fields remain omitted; quote editing is not introduced by P06.06.

## Completion notes

Local implementation and own/independent verification are complete. Independent
comparison matched twelve normalized HTTP outputs and the no-quote fingerprint.
A real old-code creation, correction and void replayed unchanged after migration;
parent reran that probe successfully. Independent quoted nine-pair and voided
history probes were retained in permanent tests and the final transfer target
passed; runtime source did not change after independent verification.

Repository validation, OpenAPI YAML/schema readback and `git diff --check` passed.
The verifier had no optional Python `jsonschema` package; it used direct field,
type, enum and decimal-pattern assertions without installing a dependency.
Full infrastructure was not repeated after test-only probe retention. RDD remains
globally off (disabled/unmanaged); no native review was started.

Maintainer approved the single-PR size exception (approximately 1,040 authored
changed lines). Local commits are in progress; no publication authorized.
Pre-existing P06.04/P06.05 documentation and untracked `.atl/` remain preserved.

Rollback boundary: the P06.06 domain, quote API/history additions and migration
are one work unit; existing stored quotes require a reviewed forward correction,
not a destructive down migration. Preserve the unrelated P06.04/P06.05 evidence.
