# P05.05 — Exact owner-scoped account balances
Add current per-account balances to the existing paginated listing, from append-only postings.
Branch: `feat/p05-05-account-balances`; baseline `5158ca7`. Delivery: exception-ok, single PR explicitly approved (approximately 500 authored changed lines). Runner: pinned Rust and disposable PostgreSQL harness.

## Specs

- S1: “Ahora comencemos con la tarea P05.05” — “Query account balances from append-only account movements, scoped to the authenticated owner; never accept a client-supplied balance or combine currencies.” Sum all signed postings including compensations without filtering by current transaction state; no postings means zero. Preserve exact symmetric Money bounds and canonical decimal strings.
- S2: “Sí” to “¿Aprobás agregar `balance` al listado existente, sin filtros históricos ni totales entre cuentas?” — “agregar `balance: "123.45"` a cada cuenta de `GET /v1/accounts`, usando su `currency` actual y conservando paginación, aislamiento y email verificado.” Preserve metadata, archive visibility, empty pages, input rejection, snapshot/no-write guarantees and existing POST behavior. No additional balance endpoint, history filters or totals.

## Tasks

- T1 [S1–S2] inline, implemented; own and independent checks passed; commit/native review pending; implement exact page-scoped aggregation, HTTP regressions and synchronized contract/docs. No writer trigger: one cohesive understood behavior. Risk: items 2/3/4 (ownership, shared listing contract and coherent snapshot).
- T2 [S1–S2] independent verification PASS S1–S2; named high-risk items 2/3/4 require fresh proof after own checks. Commit/native review pending. No publication authorized.

## Log

- L1: “Ahora comencemos con la tarea P05.05”
- L2: “Sí” — approved the balance-in-listing contract quoted in S2. Existing P05.04 five-field contract is intentionally extended only for GET; POST metadata DTO stays unchanged.
- L3: CodeGraph-first read-only map and parent spot check confirmed Money range/formatting, append-only postings including compensations, owner/date index and no approved balance HTTP contract before L2. Existing short read-only repeatable-read listing snapshot can include sums. No migration/dependency is expected. PostgreSQL SUM(bigint) is numeric: keep its exact textual integer, check domain range before formatting, never cast/wrap an overflowing sum or add row-by-row in a bounded integer.
- L4: Preserve the previously authorized, uncommitted P04.06/P04.10 partial-progress wording in master-plan.md and pre-existing .atl. These are not source changes for this task. Do not drop prior edits. P05.04 PR73 merge and full tree equality verified; archive its record only during relevant documentation closure. No remote work authorized for this feature.
- L5: Observe meaningful DB RED before source edits. Own proof covers exact signed totals, currency/owner isolation, compensations/voided events, zero, range boundaries/overflow, pagination, no writes, failures and coherent balances under concurrent posting. Reuse existing listing auth/input/POST regressions and existing creation/opening targets. Differential checks must allow only the requested balance addition; no unrequested metadata/error/POST changes. No full migration sweep required without schema changes. Native review consent and a new size decision, if needed, remain separate.

- L6: Observed RED before production edits: original six listing cases passed; both new balance cases failed because expected decimal strings were absent (Null vs123.00 /92233720368547758.07). After implementation all eight DB cases passed (17.677s). Final SQL numeric sums are checked through Money only for returned rows; lookahead does not poison an earlier valid page. Added concurrent-posting assertions to the existing coherent-snapshot regression; final checks pending.

- L7: Own suite passed repository249, fmt, Clippy, full Cargo and listing8/create7/opening8 DB (35.953s). Independent verifier PASS S1–S2, no blockers: seam suite passed, ninth scratch probe confirmed real profile confirmation → negative savings opening POST → identical replay → GET exact balance, five-field POST unchanged and no-write hashes.54 normalized baseline/current HTTP comparisons identical, both probe stdout/stderr/exit0 identical;145OpenAPIrefs resolve. Retain ninth probe as regression and rerun final normalization/checks. No production corrections; no migration/dependency changes.

- L8: Final retained-probe proof: Clippy/full Cargo passed (86passed/66DBignored); listing9 DB passed in13.645s. Parent repository249 and diff checks passed, and inspected exact owner/currency SQL and sentinel-before-Money validation. Own create7/opening8 had already passed; no runtime changes since independent PASS. No vulnerability scan/full unchanged migration sweep, no schema/dependency change. P05.05 remains unchecked pending native review.
- L9: One honest slicing pass: approximately500 authored changed lines including tests, contracts, ADR and tracking. One coherent balance-in-listing behavior is best kept with range/security/concurrency/public-flow proof and contract; a file-type split would detach proof. Recommend a fresh single-PR size exception before commits. Previous P05.04 exception does not transfer. Prior P04.06/P04.10 prose remains preserved, not silently discarded. Await size decision, then local commit and candidate-specific native review; no push/PR authorization.

- L10: “Sí” — maintainer approved a fresh single-PR size exception after disclosure of approximately500 changed lines. This is not native candidate consent or remote publication authorization. Rollback boundary: remove the GET balance aggregation/field and its contract/tests/docs while preserving P05.04 metadata listing, POST creation/opening behavior and all stored postings. Prior separately authorized P04.06/P04.10 progress wording is preserved in the same related roadmap documentation.
