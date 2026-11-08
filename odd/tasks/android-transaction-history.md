# Android transaction history — P11.06
Objective: Read paginated recorded events using the existing date/category/account filters.
Branch: `feat/p11-06-transaction-history`, base `7b587d6`; delivery: exception-ok, one future PR, no publication. Runner: Gradle shared host/iOS tests and isolated Android instrumentation.

## Specs
- S1: “Comencemos con la tarea P11.06 del plan maestro”. Roadmap: “Implement transactions list and filters.” Use existing recorded-event semantics, exact amounts, one transfer entry, stored business dates, posted/voided labels and no totals.
- S2: Maintainer confirmed “Sí” to “listado paginado y esos filtros existentes, sin ampliar el backend ni filtrar páginas incompletas en el cliente”: dates, category and account including no account. Draft/apply/cancel/reset; no default date restriction. Currency/type filters and backend changes excluded.
- S3: Maintainer confirmed “Sí” to “una única PR con excepción de tamaño para P11.06, manteniendo commits separados por comportamiento y sin publicar nada todavía”. Preserve recording entry points and authenticated lifecycle; check loading/empty/error/retry, pagination and late-result retirement.

## Tasks
- T1 — S1/S2/S3 — inline — reader complete — authenticated read adapter and common tests; commit pending.
- T2 — S1/S2/S3 — inline — pending — native browsing/filter state, screen, integration and native tests; commit pending.
- T3 — S1/S2/S3 — delegated verification (risk items 2/4: owner isolation and late asynchronous publication) — pending — independent probes after own checks, docs and closure.

## Log
- L1: User request: “Comencemos con la tarea P11.06 del plan maestro”.
- L2: User scope confirmation: “Sí”. User size-exception confirmation: “Sí”.
- L3: Clean main at 7b587d6. Existing HTTP TransactionQuery supports dates/category/account; wireframe currency/type filters are proposals only. RDD globally off; no review lifecycle. Forecast 800–1200 authored changed lines; exception approved. Work inline: one dependent behavior, no independent parallel writer units. Read-only explorer mapped prerequisites.
- L4: Planned checks: shared Android/iOS suites, assemble/lint, shared artifacts, isolated API37 native suite, repository check. No remote/backend/device access authorized. UI reads remain memory-only, clear on signed-in subtree disposal. All source artifacts in English.

- L5: T1 focused test observed RED (missing reader), then GREEN: four Android-host reader tests. Existing account-reader code unchanged; new adapter delegates wire encoding/parsing to existing repositories.
