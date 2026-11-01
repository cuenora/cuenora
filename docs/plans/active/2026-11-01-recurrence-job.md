# Execution plan: Safe daily recurrence job

- **Status:** Verified; merge pending
- **Date:** 2026-11-01
- **Roadmap task ID(s):** P07.09
- **Tracking issue / PR:** One maintainer-approved size-exception PR from `feat/p07-09-daily-job`; issue waived
- **Owner:** Agent with maintainer decisions

## Goal and user-visible outcome

An externally scheduled one-shot command generates due planned occurrences safely.
See [specification and log](../../../odd/tasks/p07-09-recurrence-job.md).

## Existing state and evidence

P07.08 provides atomic verified-owner generation through rule-local today. The API
has no job scheduler. SQLx supports close-on-drop pooled connections; PostgreSQL
session advisory locks survive individual transactions.

## In scope

One-shot binary, single-session overlap exclusion, bounded discovery, isolated
rule transactions, privacy-safe summary/errors, documented external daily schedule.

## Out of scope

Scheduler installation, deployments, HTTP/background tasks, financial posting,
new dependencies/migrations, remote operations and changes to PR91.

## Constraints and invariants

Preserve public generate_due and bearer semantics. All writes revalidate owner
verification and rule ownership under existing lock order. The lock session also
executes generation, and is discarded rather than returned to the pool.

## Steps

- [x] Observe RED for command behavior and isolated PostgreSQL job scenarios.
- [x] Implement job and shared transaction primitives; observe GREEN.
- [x] Check existing behavior, full workspace and independent risk verification.

## Acceptance criteria

- [x] One pass recovers eligible backlog across verified owners without posting.
- [x] Overlap, per-rule failure, cancellation and connection loss are safe.
- [x] Bounded scan, retries and privacy-safe CLI behavior are tested.
- [x] Existing generator and API behavior remain unchanged.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Disposable job/generation/lifecycle targets | Pass | Initial 5 job groups RED → GREEN; final job 10/10, existing generation 10/10 and lifecycle 9/9 passed | Passed |
| Rust fmt, Clippy and workspace | Pass | Final 108 passed, 0 failed, 155 intentionally ignored DB cases; applicable cases run explicitly in harness | Passed |
| Repository and independent checks | Pass | Repository/diff checks passed; S1–S4 independent PASS, 80 HTTP responses identical, parent repeated job10 + CLI unavailable probe | Passed |

## Risks, assumptions, and decisions

Risk items 2/3/4: shared authorization, existing generation and concurrency.
The maintainer approved external daily scheduling rather than an embedded worker.
Job uses bounded pages/transactions; default output contains aggregate counts only.
Maintainer approved one size-exception PR without an issue, using the current
gh session and SSH push. No merge is authorized.

## Completion notes

Implemented one job binary and shared verified-owner transaction/generation
primitives without changing the public generator or existing HTTP behavior.
Cargo metadata verifies `cuenora-api` remains the default run target.

Retained independent regression probes for backend termination, owner verification
and pause changes after discovery, physical-session disposal and unknown-lock-
result cancellation. A deterministic failed-first rule proves rollback does not
poison the following transaction. Two CLI groups cover non-database failures.
Clippy found an unnecessary vector in a retained probe; using an array resolved
it, with no suppressed warning. All final applicable checks passed.

The three relevant infrastructure targets were executed; the full approximately
25-minute infrastructure suite was not rerun because unrelated financial/schema
infrastructure is unchanged. No scheduler was installed. PR91 merged separately, and its branch/head remains
untouched by P07.09. Delivery uses updated main `0fcb7a9`, whose tree is identical
to the original `48253a7` verification baseline; tests remain applicable.

A bounded slicing pass found that extracting the compatibility refactor alone
would not bring the coherent worker/runtime/tests/docs remainder below budget.
The maintainer approved retaining one coherent size-exception PR.
P07.10 confirmation remains separate.

Implementation commit: `c9a4898eeb9aea8d6dd0a6c9cc601dd8be56a4a5`.
One PR contains the job, tests and documentation; deployment and merge remain
separate maintainer operations.
