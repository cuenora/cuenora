# P08.02 — Savings terms storage
Objective: additive savings terms and rate history storage. Branch: feat/p08-02-savings-storage.
Delivery: single-pr (maintainer explicitly requested one PR); forecast ~350 authored lines. Runner: isolated PostgreSQL Compose harness.

## Specs
- S1: “Continuemos con la siguiente tarea entonces” selects “P08.02 Migrate `savings_terms` and `savings_rate_periods`.” Store optional, owner-scoped savings-only configuration, with no accounting effects; preserve existing data.
- S2: “Sí” approves effective-dated payout-frequency/capitalization history, and “Sí” approves “hasta 6 decimales porcentuales, sin redondear entradas”. Preserve exact nonnegative nominal annual percentage inputs; reject extra scale. Existing P08.01 rules remain in docs/product/savings.md.

## Tasks
- T1 [S1–S2] inline, verified: additive schema with owner/type references, effective-date slots and history protection. Commit pending. Risk: items 1, 2 (migration and ownership constraints).
- T2 [S1–S2] inline, verified: real PostgreSQL regression tests, storage specification and execution record; independent verifier after own checks. Commit pending.

## Log
- L1: User: “Continuemos con la siguiente tarea entonces”. Follow-up approvals: effective-dated non-rate terms history and six percentage decimal places, each “Sí”.
- L2: Mapping worker confirmed composite ownership/type keys and disposable harness; nominal rate precision was previously undefined. Existing P08.01 docs and untracked .atl/ are preserved. RDD off globally; no review activation. No remote operations authorized.
- L3: Design: separate append-only effective-date streams per account for terms and rates; rates do not reset when payout configuration changes. No configuration API, calculator, posting or mid-period payout-transition implementation. Migration storage can hold incomplete terms; future forecast must require complete coverage.
- L4: RED/GREEN, compatibility, verifier and commit evidence pending. Pre-existing P08.01 documentation remains uncommitted; do not silently include it in a migration commit.
- L5: RED observed at first terms insert before migration. GREEN: storage test passed after correcting expected reverse-account SQLSTATE to existing 23001; upgrade with actual opening/movement/audit rows and fresh schema passed. No migration correction was required.
- L6: Full infrastructure suite 76/77 passed; sole failure was stale test expectation in its already-loaded process. Corrected focused rerun passed. Cargo fmt, Clippy -D warnings and workspace tests passed (173 database cases ignored by ordinary Cargo). Independent verifier S1 and S2 PASS, all three focused tests plus reverse-reference snapshot probes PASS.
- L7: Commit decision pending because P08.01 prerequisite documentation was already uncommitted. No remote operation authorized; .atl/ untouched. RDD remains off. Optional advisory: retain verifier's separate terms-only/rates-only reverse-reference probes in future test expansion.
- L8: Parent spot check repeated populated financial-history upgrade test after verifier: PASS (13.111s). Repository validator and diff whitespace check PASS. Scratch cleanup command was denied; outside scratch directories remain at /tmp/cuenora-verify-p0802.DjPkFN and /tmp/cuenora-p08-02-verify.S3g2hh.
- L9: User: “Crea una unica PR con los cambios usando ssh para el push y la sesion actual de gh. No necesitamos issue, lo apruebo así”. Authorized combined P08.01/P08.02 publication without issue; no merge. Commit evidence is the feature branch commit containing this record.
