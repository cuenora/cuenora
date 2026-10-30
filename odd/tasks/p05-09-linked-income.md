# P05.09 — Normal-account income
Record ordinary income linked to an owned normal account. Branch: `feat/p05-09-linked-income`; baseline: `a3f5255`.
Delivery: exception-ok (single PR, maintainer approved); forecast 450 authored lines. Runner: pinned Cargo and disposable PostgreSQL harness.

## Specs

- S1: “Comencemos la tarea P05.09 del plan maestro”. Roadmap: “Record ordinary income linked to a normal account; reject savings destinations on creation, edits, and later assignment.” Extend POST /v1/transactions; existing database constraints cover edits/assignment until P05.13 exposes those operations.
- S2: Preserve P05.08 accountless behavior and permanent retry equivalence; atomically post one positive movement with the event/audit/key. Reuse approved owner→account locking, verified ownership, zone/date, active-state and currency policies. These are existing repository constraints, not new product requirements.

## Tasks

- T1 (S1–S2), inline, implementation/own checks passed: RED linked tests (8 old pass/2 new fail), GREEN eleven isolated DB cases; implementation commit `a1bcfa0`. No writer trigger applies.
- T2 (S1–S2), independent verification passed; native review declined for this candidate: high-risk ownership, financial postings, shared contract and locks (items 2–4); implementation commit `a1bcfa0`; published as PR #79 against main.

## Log

- L1: User request: “Comencemos la tarea P05.09 del plan maestro”.
- L2: main at a3f5255 includes P05.08. Pre-existing untracked .atl/ preserved. Read-only explorer confirmed migration 0008 already rejects ordinary income to savings on insert/update; no migration needed. Unknown/foreign account uses generic existing 404 not_found; owned inactive/incompatible destination uses 400 invalid_request. No edit API added before P05.13.

- L3: Own proof: Cargo workspace 89 passed/85 intentionally ignored; Clippy and fmt passed; focused Compose harness passed all eleven DB cases in 12.619s. Initial archive-race test compile failed with E0716; bound its path before tokio::join!, then checks passed. Contract identifiers and old accountless fingerprint bytes retained. Implementation and docs normalized; independent verification pending.

- L4: Independent verifier passed S1/S2; full isolated infrastructure 56/56 in 1176.283s. Baseline-created accountless durable keys (omitted/null account and uppercase category) replay identically on candidate with unchanged persisted hashes. Foreign/missing account output, changed destination conflict, archive/profile-change replay and revoked verification probes passed. Adopted two permanent response/conflict comparisons; focused PostgreSQL rerun passed all eleven cases (12.491s); final Clippy passed. Full infrastructure not rerun after test-only assertion additions. Native mode on (global), candidate consent not yet requested. No remote operations.
- L5: One slicing pass: production validation/posting, backward-compatible retry behavior, tests and contract form one dependent work unit. Authored size exceeds 400 with reviewable tests and execution records; recommend a single-PR size exception, without publication authorization. Await user decision before commit. Risk: items 2, 3, 4 (ownership, shared income contract, posting/archive serialization).

- L6: User: “Sí, lo acepto, publica la rama y crea la PR”. Single-PR size exception accepted; publication/PR requested. Configured origin is cuenora/cuenora over SSH; credential/session authorization must be confirmed before remote work. Candidate native consent is separate and pending.

- L7: User chose “2”. Exact captured native decline invocation confirmed action=declined and consent=declined_this_candidate for target sha256:ffd9caefcaa9d454d76bfc58fe167476ff297ac802c7771711600996db5a295c. No native review record was created; future reviews remain enabled. Implementation commit a1bcfa0. Publication awaits explicit authorization for configured SSH and GitHub CLI session.

- L8: User answered “Sí” to configured SSH and current GitHub CLI session authorization for publishing this branch and creating the PR in cuenora/cuenora against main, without merge. Branch published; PR https://github.com/cuenora/cuenora/pull/79 created and confirmed open with type:feature,size:exception. Implementation a1bcfa0, verification/review-choice closure b2e9d50. Publication records form a passive follow-up; no production code changed after verification.
