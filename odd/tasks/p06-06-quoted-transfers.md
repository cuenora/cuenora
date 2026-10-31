# P06.06 — Optional exact transfer quotes
Objective: complete manual cross-currency recording with an optional exact quote. Branch: `feat/p06-06-quoted-transfers`; baseline: `ea5fa9f` plus preserved uncommitted P06.04/P06.05 documentation.
Delivery: exception-ok; single PR explicitly approved for approximately 1,040 authored changed lines, including preserved P06.04/P06.05 documentation. Runner: Rust 1.99.1 and disposable PostgreSQL.

## Specs

- S1: User: “Comencemos la tarea P06.06” — “Support manual SEK/EUR/USD cross-currency transfers with required sent/received amounts and an optional rate; reject unsupported currencies.” Preserve existing no-quote posting behavior and all nine currency pairs.
- S2: Apply the accepted P06.01 contract in `docs/product/currencies-and-transfers.md`: “Exact positive decimal string, destination units per one source unit, with at most 12 fractional digits.” Canonicalize spelling; checked exact arithmetic and half-even destination minor units; invalid/overflowing requests have no effects.
- S3: “Preserve the quote and both actual amounts independently”; “informational warning, not rejection”. Return quote comparison data for clients and retain the quote in transaction history. No mobile UI, automatic FX, rate editing or inferred fees. Existing edits/voids preserve the stored quote; history recomputes comparisons from current amounts. New optional JSON fields are omitted without a quote, preserving existing outputs.
- S4: Preserve permanent no-quote retry fingerprints and acknowledgements; canonical quote equivalence replays, changed quote conflicts, rollback consumes no keys. Existing ownership, same-account, amount/date, archive, audit and posting invariants remain unchanged. Additive migration does not rewrite existing financial values.

## Tasks

- T1 [S1-S4] inline; implemented and independently verified; exact domain quote, additive storage, creation/history/contract integration. No writer trigger: one dependent behavior, no independent parallel write units. Risk: items 1,3 (financial schema and existing shared transport/history). Commit: `7dd271a`.
- T2 [S1-S4] inline; own focused/workspace checks passed; deterministic RED then GREEN, focused checks and workspace suite. Commit: `7dd271a`, with T1.
- T3 [S1-S4] delegated verification; PASS S1–S4; independent financial/schema/compatibility proof. Commit: `7dd271a`.

## Log

- L1: “Comencemos la tarea P06.06”
- L2: Existing no-quote implementation covers all currency pairs, unsupported currency rejection and authority/atomic retries. The missing behavior is optional quote input/storage and discrepancy metadata. Mapping worker returned paths-injected.
- L3: The phrase “No database rate type, dependency or migration is introduced by this contract” describes P06.01 documentation-only scope; the same document explicitly assigns quote storage to P06.06. It does not forbid an additive implementation migration. Store canonical decimal text and reuse checked `Money::from_minor_ratio`; introduce no dependency.
- L4: Existing uncommitted P06.04/P06.05 documentation and pre-existing `.atl/` must remain intact. No remote operations authorized. Domain prerequisite can stand independently; storage/HTTP/history integration must remain one work unit. Resolve delivery strategy before commits exceeding the review budget.

- L5: RED observed: domain import `QuotedRate` absent; three new HTTP groups returned 400 instead of 201, while all sixteen prior groups passed. Initial GREEN exposed the intentionally obsolete valid-quote rejection case; replaced only that case with the still-unsupported `rate` alias. New valid/invalid/retry groups now pass.
- L6: Implemented exact canonical domain quote and flattened optional metadata, additive migration0016, quote-sensitive hashes only when present, history comparisons and correction overflow protection. No-quote hashes/responses unchanged. Actual amounts alone determine postings; PATCH still rejects quote input.
- L7: Own four isolated targets passed in93.345s: nineteen transfer groups, populated quote migration, five history groups and ten correction groups. Domain quote tests passed; ordinary workspace suite passed (database tests explicitly ignored). Clippy and repository/diff checks passed; final post-normalization check pending. RDD remains globally off, no native review started.

- L8: Independent verifier PASS S1–S4: full64-group isolated infrastructure suite passed in1358.740s; fourteen invalid quotes, all nine quoted currency pairs, checked half-even cases, unchanged rejection hashes, late audit rollback/reuse, quote preservation after correction/void and correction overflow passed. Twelve normalized HTTP responses and a no-quote request fingerprint matched baseline. Real baseline-created/edited/voided/archived transfer replayed unchanged after migration0016; parent reran that upgrade probe successfully. No blockers or correction round.
- L9: Retained unique nine-pair/voided-history probes in permanent tests and added null/absent no-quote equivalence. Parent final nineteen-group target passed in15.939s, Rustfmt/Clippy passed. Runtime source unchanged after independent proof; full64-group suite not repeated after test-only additions. Python jsonschema unavailable; direct OpenAPI field/type/pattern assertions passed. ADR-033 records storage/arithmetic/compatibility tradeoffs.
- L10: One slicing pass: the exact domain type is an independent prerequisite, but separating it leaves the dependent storage/API/history/tests/docs unit above400 lines. Keep behavior and its tests together; recommend one single-PR size exception rather than artificial splits. No commits or publication before the size decision. Own/independent evidence is complete; delivery remains pending.
- L11: Delivery forecast now approximately1,040 authored changed lines (1,035 measured before this log entry), including preserved P06.04/P06.05 docs. Parent awaiting explicit single-PR size exception; no remote operation is part of that consent.

- L12: “Sí” — maintainer accepted the single-PR size exception after disclosure of approximately 1,040 authored changed lines, explicitly keeping delivery local. Proceed with local Conventional Commits only; no push, PR creation or merge authorized.

- L13: Local implementation and all regression evidence committed as `7dd271a` (`feat(transfers): record optional exact transfer quotes`): 22 files, 995 insertions and 48 deletions (1,043 authored changed lines). T1–T3 complete. RDD disabled/unmanaged; no assessment or native review started. No push/PR/merge performed or authorized; `.atl/` remains the only pre-existing untracked surface. Final passive delivery documentation follows.
