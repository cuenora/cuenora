# Execution plan: P05.05 account balances

- **Status:** Implemented; own/independent checks passed; single-PR size exception approved; native review pending
- **Date:** 2026-10-29
- **Roadmap task ID(s):** P05.05; operation-specific P04.06/P04.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Add a canonical decimal-string `balance` to every account in `GET /v1/accounts`,
paired with its existing currency. Requirements and task evidence are in the
[task log](../../../odd/tasks/p05-05-account-balances.md).

## Scope and invariants

Sum append-only movements, including compensations even for voided events.
No movements means zero; negative balances are valid. Keep owner/currency
boundaries, verified email, pagination, archived visibility and the existing
read-only snapshot. Reject unrepresentable totals safely, without partial pages,
rounding, saturation or writes. Do not change POST responses, add history filters,
combine currencies, introduce migrations or implement future financial writes.

## Steps and verification

- [x] Observe meaningful RED for the approved response and arithmetic.
- [x] Implement exact bounded-page aggregation, contract and documentation.
- [x] Run formatting, Clippy, ordinary Cargo and isolated listing/create/opening checks.
- [ ] Complete independent high-risk verification and applicable native review.

Risk: items 2/3/4 — ownership, shared listing contract, concurrent read snapshots.
Source-mutating normalization precedes proof and native candidate freeze.

## Completion notes

Meaningful RED: both balance cases failed on absent fields while six original
listing cases passed. Final ordinary Cargo: 86 passed, 66 DB tests ignored.
Repository (249 files), fmt and Clippy passed. Disposable listing9 passed after
retaining the independent public-opening/replay/GET regression; existing create7
and opening8 passed in the seam suite. Independent verification PASS S1–S2,
54 baseline/current HTTP comparisons unchanged, 145 OpenAPI references resolved.
No production correction, migration or dependency change. Native review remains
pending. No full unchanged migration sweep or vulnerability scan was run.
Email verification runtime remains a separate product prerequisite.
