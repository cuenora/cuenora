# P04.05 — Authenticated identity middleware
Branch: `feat/p04-05-authenticated-identity` · Base: `b1ed769` · Delivery: `exception-ok`; maintainer approved one coherent oversized PR without publication. Original forecast 500–650 authored lines; observed approximately 800.
Runner: pinned Rust 1.99.1 and disposable PostgreSQL. Route: inline implementation; independent security verification. No publication authorized.

## Specs

- **S1:** User: “Comencemos la tarea P04.05”. Roadmap: “Implement authenticated identity middleware.” Validate existing opaque access credentials and attach server-derived user/session identity to protected handlers.
- **S2 (existing constraints):** Authenticate only unexpired access digests belonging to non-revoked sessions. Never trust caller owner IDs, log secrets, mutate identity/session/financial data, or equate authentication with financial authority. Pending and verified users can authenticate. Ownership, logout/global revocation, financial routes and email workflows remain later tasks.
- **S3 (implementation assumption):** Supply reusable, route-scoped middleware with a test-only protected handler; do not add a production endpoint or change existing public routes. Reject missing, malformed, unknown, expired, revoked or refresh credentials safely; storage failures fail closed. Document integration and observed validation only.

## Tasks

- **T1 · S1–S3 · inline · pending:** Middleware, public-interface tests, isolated PostgreSQL harness and synchronized documentation/ADR in one security behavior unit. No writer trigger; one dependent behavior. Risk: item 2 (authentication boundary). Commit pending; runtime RED observed (two transport and three DB groups returned 500 through the no-auth scaffold); GREEN observed (2 transport + 3 isolated DB groups).
- **T2 · S1–S3 · delegated verification · pending:** Independent security proof in an external scratch copy after own checks; parent spot check, native assessment/review when applicable, and passive closure. Verification trigger: item 2. Commit pending.

## Log

- **L1 (2026-10-27):** User: “Comencemos la tarea P04.05”.
- **L2:** Main `b1ed769` has no tracked edits; preserve pre-existing untracked `.atl/`. Read-only explorer mapped existing access schema, token generation and router; parent checked migration 0009, module exports and test harness. Access lookup/middleware are absent; health/register/login/refresh are public. No new schema or dependencies needed.
- **L3:** Implement inline with test-first proof. Typed identity is populated only by successful database validation. Lookup is a read-only statement: expiry and committed family revocation are checked in its database snapshot; revocation cannot retroactively cancel a handler already admitted. No cache or session renewal. Protect future routes explicitly, not public auth/liveness routes. Forecast requires delivery strategy resolution before a work-unit commit.

- **L4:** Implemented route-scoped bearer middleware and typed identity, strict header parsing, complete-secret digest lookup, strict database expiry and committed-family revocation snapshot checks. No production router/contract/schema/dependency changes. Own checks: focused transport 2 passed; isolated PostgreSQL 3 groups passed (16.241 s harness); fmt, Clippy with warnings denied, full workspace 52 passed including 11 doctests and 31 intentionally ignored DB cases; repository validation 196 Git-visible files passed. Ignored DB cases are exercised by focused harnesses, not claimed passed by Cargo. Added ADR-018 and current integration/status docs. Independent security proof pending.

- **L5:** Independent odd-verify passed S1–S3 with no blockers: 2 transport groups, 17 PostgreSQL cases across four focused harness methods (88.793 s), and 24 exact baseline/candidate public-route status/header/body matches, both processes successful. Verified unchanged original production router/helpers/config/schema/contracts/dependencies. Retained its nonduplicate cookie/query/body valid-secret rejection probes in the existing DB group; no production changes. Parent reran final fmt/Clippy/workspace (all passed) and focused middleware PostgreSQL (3 groups, 16.330 s), including those probes. Full 45-test financial infrastructure suite intentionally not run.
- **L6:** Approximately 800 authored lines form one coherent security behavior with its tests/docs; no cosmetic reductions or artificial test/code split. Implementation and independent proof pass, but work-unit commit/delivery strategy and native candidate review remain pending. Ask maintainer for a single-PR size exception rather than borrowing the P04.04-only exception. No commit, remote operation or master checkbox closure yet. Risk: item 2 (authentication boundary).

- **L7 (2026-10-27):** User: “Sí” to “¿Autorizás mantenerla en una sola PR con excepción de tamaño, sin publicarla todavía?” This explicitly approves the P04.05 single-PR size exception only. No publication, remote credential/session use or merge authorized. Continue the local work-unit commit and native security assessment; candidate-specific review consent remains separate.
