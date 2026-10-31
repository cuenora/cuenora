# P06.03 / P06.07 — Atomic transfer creation
Objective: one atomic transfer with permanent retries. Branch: `docs/p06-01-transfer-contract`; baseline: `217ac81182a06699c320bd96502271db51f1bdd2` plus uncommitted P06.01/P06.02 prerequisites.
Delivery: exception-ok, single PR explicitly approved; approximately 1,670 authored changed lines including P06.01/P06.02 prerequisites. Runner: pinned Rust 1.99.1 and disposable PostgreSQL harness.

## Specs

- S1: “Hagamos la tarea P06.03 entonces” — “Implement a single atomic SQL transaction for transfers.” Event, actual source debit, actual destination credit and minimal audit commit together. The nine existing public P06.02 cases must turn GREEN. Existing ownership, verification, date, currency, active-account and same-account invariants apply; no security bypass while dedicated later tasks remain pending.
- S2: “Sí” to “¿Autorizás incluir P06.07 en este cambio?” — “Implement `Idempotency-Key` for transfer POST requests.” Independent permanent owner-scoped transfer namespace; canonical request hash, unchanged original acknowledgement on equal retries (including after corrections), conflict on different content, no duplicate financial effects.
- S3: Preserve existing income/expense transport, outputs and retry namespaces. Reuse `normalize_uuid`, `today_at`, existing transaction/movement/audit schema and disposable harness. No quoted-rate field/storage, automatic FX, new dependencies, reporting or phase-completion claim. OpenAPI and run instructions must distinguish the shipped no-quote slice from remaining P06.06 work.

## Tasks

- T1 [S1,S2,S3] inline; implemented and verified; one dependent behavior, no writer trigger. Risk: items 1,2,3,4,5 (retry schema, ownership, shared POST transport, serialized retries and additive migration). Commit `f8a186f`.
- T2 [S1,S2,S3] inline; verified; public acceptance/regression tests, focused PostgreSQL and ordinary workspace checks. Prior P06.02 RED observed: nine HTTP 400 instead of 201. Commit `f8a186f`, with T1.
- T3 [S1–S3] delegated verifier after own checks; PASS S1–S3; independent security/persistence/concurrency proof required. Parent final sixteen-group target rerun passed. Commit `f8a186f`.

## Log

- L1: “Hagamos la tarea P06.03 entonces”
- L2: Parent proposed including P06.07 because exposing financial writes without retry protection violates repository invariants. User answered “Sí”.
- L3: Existing P06.01/P06.02 changes and untracked `.atl/` preserved. CodeGraph mapping confirmed strict income/expense parser, owner-first locking and immutable income/expense key pattern. Implement inline; no independent writer units. Mapping worker returned paths-injected.
- L4: P06.06 quoted-rate storage/precision runtime remains out of scope; unknown quote fields reject. No remote publication authorized. RDD globally off as last observed; do not enable it.

- L5: Added 0015 transfer keys, owner-first atomic creation, strict transfer-only JSON branch and OpenAPI contract. No quote field accepted. Existing normalized UUID/date helpers reused; no writer trigger.
- L6: Existing/new tests observed RED before source writes. Initial GREEN attempt exposed a pre-existing P06.02 helper sending JSON null on strict GET /accounts; fixed test transport to send empty GET. Another new assertion incorrectly expected 403 for missing zone; current public contract is 409, so test/docs corrected without changing existing error behavior.
- L7: Final own focused checks passed: fourteen transfer DB groups plus populated migration preservation; all nine P06.02 posting/balance assertions reached and passed. Pinned Rustfmt, Clippy, full ordinary workspace Cargo, repository validation and OpenAPI YAML/input-alternative checks passed. Full DB suite and independent scratch/differential probes pending.

- L8: Independent verifier PASS S1–S3: 63 disposable infrastructure groups passed (1299.300s), plus destination-credit/retry-insertion rollback, foreign ownership on both legs, distinct keys and unverified replay denial. Twenty-six normalized income/expense responses were identical to baseline, with unchanged full financial hashes on rejections. Initial shared Cargo cache reused baseline artifacts; independent target dirs and scoped recheck corrected the verification setup, not source.
- L9: Parent retained verifier probes as deterministic regression tests; no git-history or external-output dependency. Expanded transfer target to sixteen groups and full financial snapshots. Fixture-only correction captures snapshot after intentional account archival, avoiding comparison to pre-fixture metadata. Final target passed; Rustfmt/Clippy passed and ordinary Cargo passed (91 ordinary/doctests, 123 ignored). Runtime source unchanged after independent proof. Full 63-group suite not repeated after test-only additions.
- L10: P06.02/P06.03/P06.07 now reflect verified local behavior; P06.04–06/P06.08–10 remain separate tasks. No quote accepted. One slicing pass: P06.01 passive docs can stand apart; the atomic/retry runtime, schema, tests and contract remain inseparable and exceed 400 lines. Size exception recommended; no commit/push/PR before the decision. Scratch copies/caches will be removed after evidence is recorded; pre-existing `.atl/` preserved.

- L11: “Sí” — maintainer explicitly accepted the single-PR size exception after disclosure of approximately 1,670 authored changed lines, including P06.01/P06.02. No publication authorized. Native RDD remains globally off; no review started. Verification scratch/caches/logs were removed; `.atl/` preserved. Proceed with local Conventional Commits only.

- L12: Implementation/prerequisites committed as `f8a186f` (`feat(transfers): record atomic transfers with permanent retries`), 17 files, 1,649 insertions and 24 deletions (1,673 authored changed lines). T1–T3 complete; approved single-PR exception. Native RDD disabled/unmanaged, no native assessment/review started for the commit. Final passive documentation records the verified commit; no runtime changes. No push/PR/merge performed or authorized.
