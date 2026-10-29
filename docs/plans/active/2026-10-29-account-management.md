# Execution plan: P05.06 account management

- **Status:** Implemented; own/independent checks passed; single-PR size exception approved; native review pending
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.06; operation-specific P04.06/P04.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and approved behavior

Owners can rename, archive zero-balance accounts and reactivate archived accounts.
Archive preserves queryable history/balance and prevents future income, expense
and transfer postings. Historical corrections remain a separate policy.
See the [task specification](../../../odd/tasks/p05-06-account-management.md).

## Scope and invariants

Add strict PATCH /v1/accounts/{id} with name/status metadata only. Reuse the name
policy; never change owner/type/currency, financial records or creation replay.
Require current verified email, derive ownership from bearer identity and keep
foreign/missing resource errors generic. Serialize exact zero-balance checks with
state changes. Repeat assignments preserve timestamps. No migration, dependency,
new financial posting endpoint, balance mutation or history deletion.

## Steps and acceptance

- [x] Observe meaningful RED for management through authenticated HTTP.
- [x] Implement atomic owned metadata transitions with bounded validation.
- [x] Verify name/archive/restore semantics, no unauthorized writes, replay and concurrency.
- [x] Synchronize OpenAPI, product rules, future posting lock contract and tests.
- [ ] Run own checks, independent verification and applicable native review.

Risk: items 2/3/4 — ownership, API/creation replay compatibility, zero-check races.

## Verification and completion notes

Observed RED before implementation: authenticated PATCH404vs200. Final own
checks passed repository255, fmt, Clippy and Cargo87passed/73ignored. Isolated
management7/listing9/create7/opening8/profile3 passed; normal ignored tests are
not DB proof. Independent PASS S1–S4,11 DB-backed and18 direct baseline/current
comparisons identical,162OpenAPIrefs resolved. Retained legacy restoration and
original nonzero opening replay-after-archive regression passed. Test-only global
row-count assumption fixed by owner scoping; no runtime correction required.
No full unchanged migration sweep or vulnerability scan (no schema/deps change). Keep verification/recovery runtime and historical
correction policy explicitly pending. No remote publication authorized.
