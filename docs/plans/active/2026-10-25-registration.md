# Execution plan: Pending user registration

- **Status:** Active — stacked-to-main delivery strategy approved
- **Date:** 2026-10-25
- **Roadmap task ID(s):** P04.02; hashing prerequisite brought forward from P04.03
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and implementation agent

## Goal and user-visible outcome

Persist an unverified user and a safely hashed credential through the local API.
Login, session issuance, verification email/challenges and financial authority are
not part of this endpoint.

## Existing state and evidence

- API currently exposes only `GET /health/live`.
- Migrations 0001–0008 provide identity and financial storage; no schema change is planned.
- No application PostgreSQL client, JSON registration contract, hashing library or API/database integration harness exists yet.
- Approved identity behavior is recorded in the identity strategy and ADR-004/005.

## In scope

- Email normalization/validation preserving the original input and comparison policy.
- Approved 15–128-character password policy, spaces/Unicode, no composition rules, common-password rejection and salted Argon2id storage.
- Bounded local HTTP registration, atomic user/credential persistence and safe duplicate/error responses.
- Disposable PostgreSQL public-interface tests, OpenAPI and verified setup documentation.

## Out of scope

Login, tokens/sessions, email verification/recovery/delivery, financial routes,
deployment, production readiness, migrations and remote publication.

## Constraints and invariants

Follow root `AGENTS.md`; preserve original liveness output and monetary-domain
independence. User IDs and verification state are server-owned. Do not store/log
plaintext passwords or leak registered addresses. Do not modify credentials on
duplicate requests. Rejected/failed requests must leave persisted state unchanged.

## Steps

- [x] Resolve review-slice strategy: stacked-to-main; no publishing is implied.
- [x] T1: RED/GREEN, independent verification, audit and native four-lens review passed; exact acknowledgement confirmed.
- [ ] T2: Adapter RED/GREEN, full suite, audit and independent verification passed; native review pending. HTTP/configuration still pending.
- [ ] T3: Synchronize documentation, complete own checks and independent verification.
- [ ] Run applicable native assessment/review with candidate-specific consent.

## Acceptance criteria

- [ ] S1–S5 in `odd/tasks/p04-02-registration.md` are verified.
- [ ] Atomic persistence, safe duplicates and failure rollback are exercised through HTTP against isolated PostgreSQL.
- [ ] Hash verification, independent salts and approved password/email edge cases are tested.
- [ ] OpenAPI/examples/setup reflect actual behavior and pending authentication work.
- [ ] Required checks have observed outcomes; P04.02 remains unchecked until then.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Cargo fmt/Clippy/workspace tests | Pass without warnings | T1/T2A: 25 tests, fmt and Clippy passed; 4 DB cases explicitly executed through harness | Passed for implemented slices |
| Disposable PostgreSQL HTTP integration | Persisted success, duplicate safety, failure rollback, policy and liveness cases | Harness not implemented | Pending |
| `python3 scripts/check_repository.py` and diff checks | Pass | Passed; 165 repository files checked for T2A | Passed |
| Independent security/dependency/concurrency verification | No reproducible candidate-caused blockers | T1/T2A probes and audits passed; trigger rollback, 16-way race and duplicate snapshots verified | Partial feature; HTTP pending |
| Full PostgreSQL infrastructure suite | All existing and registration cases pass | 37 tests passed, 751.422 s; subsequent strengthened focused cases passed, 12.543 s | Passed |

## Risks, assumptions, and decisions

Risk items 2 (credentials), 4 (async hashing/duplicate races), 5 (new dependencies).
User approved Argon2id and the password policy. ADR-009/010 record the selected
parameters, offline common-password source and persistence failure boundary.
Forecast 700–1,000 authored lines excluding generated lock changes;
stacked-to-main strategy approved. First slice implements credential primitives
only (ADR-009); registration endpoint remains pending. Generated Cargo.lock growth
is tracked separately from authored review lines; a publication-size exception
may be required for the inseparable lockfile. No PR is authorized yet.

## Completion notes

T1 local commit: `97961c7611afbdfb69d1e6187596e76f4a4d77a7`; native review acknowledged.
T2 persistence slice is in progress; HTTP registration remains unimplemented.
In progress. The ODD feature document is the task/specification logbook. Keep this
execution record active until verification and eventual merge are confirmed.
