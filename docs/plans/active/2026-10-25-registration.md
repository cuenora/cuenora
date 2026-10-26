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
- [x] T1: RED observed, policy tests GREEN; independent debug/release verification and dependency audit passed. Native review pending.
- [ ] T2: Observe persisted HTTP tests RED, implement GREEN and preserve liveness.
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
| Pinned Cargo fmt/Clippy/workspace tests | Pass without warnings | T1: 4 policy + 21 existing tests, fmt and Clippy passed | Passed for T1 |
| Disposable PostgreSQL HTTP integration | Persisted success, duplicate safety, failure rollback, policy and liveness cases | Harness not implemented | Pending |
| `python3 scripts/check_repository.py` and diff checks | Pass | Not run for this feature | Pending |
| Independent security/dependency/concurrency verification | No reproducible candidate-caused blockers | T1: debug/release probes and cargo-audit passed; concurrency/HTTP pending T2 | Partial feature; T1 passed |

## Risks, assumptions, and decisions

Risk items 2 (credentials), 4 (async hashing/duplicate races), 5 (new dependencies).
User approved Argon2id and the password policy; precise technical parameters,
offline common-password screening source and failure contract remain to be
documented. Forecast 700–1,000 authored lines excluding generated lock changes;
stacked-to-main strategy approved. First slice implements credential primitives
only (ADR-009); registration endpoint remains pending. Generated Cargo.lock growth
is tracked separately from authored review lines; a publication-size exception
may be required for the inseparable lockfile. No PR is authorized yet.

## Completion notes

In progress. The ODD feature document is the task/specification logbook. Keep this
execution record active until verification and eventual merge are confirmed.
