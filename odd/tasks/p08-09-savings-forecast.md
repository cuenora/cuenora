# P08.09 — Explainable savings forecast endpoint
Branch: `feat/p08-09-savings-forecast`. Delivery: exception-ok (single PR, maintainer approved); forecast ~750 authored lines; no remote operations.
Verification: pinned Cargo workspace checks and disposable PostgreSQL HTTP tests. Risk: item 2 (ownership), item 4 (consistent snapshot).

## Specs

- S1: User: "Continuemos con la siguiente tarea entonces P08.09". Roadmap: "Add an explainable forecast endpoint with breakdown." Add a verified-owner, read-only savings forecast HTTP endpoint with exact amounts, persisted terms/rates and period breakdown; never post financial events.
- S2: User approved "usar el saldo registrado al inicio y mantenerlo constante, salvo la capitalización configurada, mostrando esa suposición en el desglose" with "Sí". No predicted deposits or withdrawals.
- S3: User approved "si el intervalo cruza un cambio de frecuencia o capitalización, el endpoint lo rechazará explícitamente" and "los cambios de tasa sí se aplicarían normalmente" with "Sí". Do not reset accumulated interest at rate changes.
- Implementation assumptions: GET `/v1/accounts/{id}/savings-forecast`, explicit inclusive `start_on`/`end_on`, at most 366 days, start today or later in the confirmed profile zone. Balance is the current posted snapshot, held flat through the requested window. An arbitrary start produces a disclosed partial first period, not an estimate of interest accrued before that date. Missing initial terms/rate coverage is an explicit conflict. Active/archived owned savings accounts may be read. No client monetary input or defaults for missing terms. Return period exact minor-unit ratios and rounded decimal-string estimates, applicable rates, policy metadata and limitations. No receipt reconciliation or historical replay.

- S4: User correction: "No, incluye los cambios de las 3 tareas en el mismo PR". Include P08.07, P08.08 and P08.09 together in the single size-exception PR. This supersedes earlier exclusion of P08.07/P08.08 from publication.

## Tasks

- T1 | S1-S3 | inline (bounded map supplied; one coherent endpoint) | complete: five disposable PostgreSQL HTTP groups fail against missing route (absent protected-route no-store header) | commit `16e61ac`
- T2 | S1-S3 | inline (no parallel writer reason) | complete: endpoint/contract/docs; seven forecast and three configuration HTTP groups GREEN | commit `16e61ac`
- T3 | S1-S3 | inline + independent verifier (risk items 2/4) | verified: full checks and independent S1-S3/items 2/4 probes passed; commit authorized | commit `16e61ac`

## Log

- L1: Original request: "Continuemos con la siguiente tarea entonces P08.09".
- L2: User confirmed flat future balance with "Sí", then explicit unsupported terms-transition rejection with "Sí".
- L3: Existing pure calculator handles consecutive supplied balances/rates with fixed terms. API histories are separate immutable streams; approved policies require no invented coverage. Existing account listing uses repeatable-read, read-only snapshots. Preserve all pre-existing uncommitted P08.07/P08.08 tests/docs and untracked `.atl/`; do not include them in a P08.09 commit.
- L4: Test-only P08.07/P08.08 changes were already verified (131 passed, 184 ignored) before this feature. RDD globally off. One coherent high-risk endpoint is the work unit; any size exception/chain decision is required before its commit.

- L5: RED observed: disposable forecast harness ran five HTTP groups against baseline; all failed because the route/protected-route headers are absent. Existing common snapshot helper will also be reused to avoid unused test infrastructure.

- L6: GREEN: seven disposable forecast HTTP groups and three existing configuration groups passed. Controlled table-lock probe proves pre-write snapshot result followed by updated snapshot result. Three OpenAPI structural tests pass (new route guard was RED before contract additions). Full workspace: 131 passed, 0 failed, 191 ignored. Formatting, Clippy and repository validation passed.
- L7: Added a conservative 4096-digit endpoint compounding budget with explicit 422 rejection and shorter-window recovery; accepted persisted rates and pure-domain numeric ranges remain unchanged. No normalization or financial mutations occur in GET.

- L8: Independent verifier PASS for S1-S3 and risk items 2/4 in isolated baseline/candidate scratch. Adopted its intra-period rate-switch/actual-owner guard probe as an eighth committed-source regression (not yet committed in Git). Final eight-group HTTP run passed; final Clippy/workspace tests passed (131 passed, 0 failed, 192 ignored). Parent spot checks OpenAPI and final HTTP run passed.
- L9: Implementation complete; maintainer subsequently approved the size exception (L10) because this indivisible behavior plus tests/contracts exceeds ~400 lines. Recommend one coherent unit with explicit size exception rather than a nonfunctional split. No push/PR/merge authorization. P08.07/P08.08 test/doc edits must remain outside the new feature commit.

- L10: User: "Crea una unica PR con excepcion de tamaño usando ssh para el push y la sesion actual de gh. No necesitas issue, lo apruebo así". One P08.09 PR, size exception, SSH push and current gh session authorized; no merge.

- L11: Implementation commit `16e61ac` contains only P08.09: 1218 additions, 2 deletions. Existing P08.07/P08.08 changes remain unstaged. All functional verification recorded above; remote CI remains pending.

- L12: User correction verbatim: "No, incluye los cambios de las 3 tareas en el mismo PR". Include all remaining verified P08.07/P08.08 tests and documentation on the same branch/PR; preserve only unrelated `.atl/`. The final workspace verification already included these exact tests and documentation.
