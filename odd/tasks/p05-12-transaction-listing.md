# P05.12 — Transaction listing
Owner-scoped financial history. Branch: `feat/p05-12-transaction-listing`; baseline: `d38f894`.
Delivery: exception-ok; forecast 850 authored lines, one cohesive read behavior; runners: pinned Cargo, isolated PostgreSQL harness.

## Specs

- S1: «Comencemos con la tarea P05.12 del plan maestro». Roadmap: “Implement paginated transactions and date/category/account filters.” Provide GET /v1/transactions with bounded cursor pages and combinable date/category/account filters, isolated to the authenticated owner.
- S2: «Incluimos ingresos, gastos, transferencias, saldos iniciales e intereses acreditados». Include the five stored kinds, never planned occurrences. Preserve each event's exact amounts/currencies and posted/voided state without inferring balance effects.
- Engineering assumptions: inclusive `date_from`/`date_to` on stored business dates; descending `occurred_on,id`; default50/max100; `account_id=none` selects accountless events, UUID selects either transfer side; omitted filters include everything. Reads require verified email, not a confirmed profile zone. Foreign/unknown filter references and cursors return identical generic 400 errors. No cross-request snapshot guarantee.
- Checks: public HTTP PostgreSQL tests for pagination/all kinds/filter combinations/ownership/rejections/unchanged storage; existing POST compatibility; pinned fmt, Clippy, workspace tests, repository checks; independent verification for security item 2 and shared HTTP item 3.

## Tasks

- T1 (S1,S2): inline; completed, own checks passed; implement listing with tests and synchronized contract/docs. Writer trigger absent: one cohesive behavior. Risk: item 2 (owner isolation), item 3 (shared transaction route/error surface). Commit: `1b1b526`; RED: 5 failures before GET (405/missing history); GREEN: all 5 isolated HTTP tests passed.
- T2 (S1,S2): independent verification; passed S1/S2, no blockers. Size exception approved; native review skipped for this candidate by maintainer choice, future reviews enabled. Implementation: `1b1b526`.

## Log

- L1: User: «Comencemos con la tarea P05.12 del plan maestro».
- L2: User: «Incluimos ingresos, gastos, transferencias, saldos iniciales e intereses acreditados».
- L3: Read-only map confirms five stored event kinds and no existing transaction GET. Existing account listing supplies bounded keyset/read-only snapshot precedent. Preserve untracked `.atl/`. No remote operation authorized.

- L4: Five isolated history tests RED before implementation (after correcting a fixture profile column), then GREEN. Pinned fmt, Clippy with -D warnings, workspace tests, repository check and parsed OpenAPI/local refs passed. Ordinary Cargo ignores database tests. GET is read-only; POST implementation unchanged except method composition.

- L5: Independent verification passed S1/S2, focused HTTP cases, all 59 infrastructure cases (1205.301s), repository check, 11 identical normalized baseline/candidate POST output/state lines, and extra empty/uppercase/revoked/expired/body/filter/hash probes. A transient DB setup failure occurred before differential probes; a fresh isolated run passed.
- L6: Own Cargo: 90 passed, 97 ignored database cases (run separately); fmt and Clippy passed. Optional extra JSON-Schema validator absent; OpenAPI YAML/local references passed. No dependency installed.
- L7: One slicing pass found one cohesive history read with shared predicates/response/proof; source/tests/docs-only splits are not independent behavior. Actual authored diff approximately 1,140 lines. Recommend size exception before commit. Native review enabled, not started; no remote authorization.

- L8: User answered «Sí» to the single-PR size exception (~1,140 lines). Local work-unit commit authorized by ODD; remote push/PR creation remain unauthorized. Rollback boundary: remove the new history module/GET method and its associated tests/contract/docs; retain all existing creation logic and schema.

- L9: User answered «2» (Skip this time). Exact native decline returned action=declined, consent=declined_this_candidate and matching target sha256:ecee8ac79310e026d8ab273a005810ef6d8b46e1251e8aabc24fe648a61d8451. No review record/approval created; future reviews stay enabled. Source committed in `1b1b526` (1139 changed lines). Publication remains unauthorized.
