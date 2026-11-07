# Execution plan: Shared resource repositories

- **Status:** Implemented and verified locally; size exception approved; local commits in progress
- **Date:** 2026-11-07
- **Roadmap task ID(s):** P10.08
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Provide typed shared account and transaction repositories for all existing resource
operations, plus deterministic test fakes. [Task log](../../../odd/tasks/p10-08-repositories.md).

## Existing state and evidence

ADR-050 supplies independent domain models, ADR-051 exact amount adapters and
ADR-052 transport results. OpenAPI defines account list/create/update and
transaction list/create/edit/void. There are no resource repositories yet.

## In scope

Typed commands, queries, domain mapping, HTTP adapters, fakes, shared tests and docs.

## Out of scope

Backend changes, UI, login/refresh/logout orchestration, local persistence,
automatic retries, real backend requests and cross-language fixtures (P10.11).

## Constraints and invariants

Explicit immutable owner/bearer context; no inferred authorization. Caller owns
client lifecycle and mutation keys. Exact decimal strings, nullable patch states,
single-operation transfers and server-only financial authority remain intact.

## Steps

- [x] Shared infrastructure and decision record.
- [x] Account behavior and fake.
- [x] Transaction behavior and fake.
- [x] Integrated tests, builds, independent check and roadmap evidence.

## Acceptance criteria

- [x] Every existing account/transaction operation is accessible and tested.
- [x] Failures and cancellation remain typed/observable without diagnostic leaks.
- [x] Both shared test targets and artifacts verify with no host/backend wiring.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Shared focused tests | RED then GREEN | Missing-class RED, then GREEN; 3/4 independent regression probes failed before correction, all seven final regressions pass | Passed |
| Android-host + iOS simulator tests, allTests | Pass | 58 host / 59 simulator, zero failures/errors/skips; allTests succeeds | Passed |
| Android AAR and two iOS frameworks | Build | All three build with the final production source | Passed |
| `python3 scripts/check_repository.py` | Pass | Repository validation passes | Passed |
| Independent security/contract check | No blockers | Three original blockers corrected; targeted 7/7 recheck passes. Baseline 32/32 and existing candidate test identities/outcomes match | Passed |

## Risks, assumptions, and decisions

Credential forwarding and patch/idempotency semantics passed independent checking.
Native review is disabled globally. The maintainer approved one future PR with
a size exception and local work-unit commits. No remote operation is authorized.

## Completion notes

All existing resource operations and fakes are implemented. Independent probes
found and pinned three defects: Money currency discarded by PATCH, insufficient
create-response correlation and quoted boolean acceptance. PATCH now uses a
currencyless exact DecimalAmount, creation correlates supplied event fields and
comparison booleans require true JSON booleans. Independent targeted recheck
passed; its additional probes were retained as workspace regression tests.

Parent spot-check of all seven regression methods and final repository checker
passed. Final shared totals are 58 host / 59 simulator, zero failures/errors/skips;
allTests and Android AAR/two iOS frameworks passed. Existing Native cache/cast
warnings remain non-fatal. No live backend, physical-device, host UI, native secure
storage service rerun or production security claim is made. Those integrations
were unchanged. Baseline/candidate shared tests preserve existing behavior.

Native review remained disabled globally; assessment was high by explicit security
escalation, not approval. The maintainer approved a single future PR with a size
exception (roughly 1.6k authored lines). Local work-unit commits are in progress;
push and PR publication remain unauthorized. Temporary verification snapshots/logs were removed after
recording results; no tool cache or scratch files are included.
