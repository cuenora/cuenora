# Execution plan: Pending user registration

- **Status:** Implemented locally — functional verification and native review acknowledged; no publication
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
- [x] T2: Adapter RED/GREEN, full suite, audit and independent verification passed; native review acknowledged. HTTP/configuration implemented; own and independent verification passed.
- [x] T3: Contract/docs synchronized; own checks, independent verification and audit passed.
- [x] Native reviews completed with candidate-specific consent; exact acknowledgements confirmed.

## Acceptance criteria

- [x] S1–S5 in `odd/tasks/p04-02-registration.md` are verified.
- [x] Atomic persistence, safe duplicates and failure rollback are exercised through HTTP against isolated PostgreSQL.
- [x] Hash verification, independent salts and approved password/email edge cases are tested.
- [x] OpenAPI/examples/setup reflect actual behavior and pending authentication work.
- [x] Required functional checks have observed outcomes; P04.02 checked without completing Phase 04.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned Cargo fmt/Clippy/workspace tests | Pass without warnings | T2B: 31 tests, fmt and Clippy passed; eight DB cases explicitly executed through harness | Passed |
| Disposable PostgreSQL HTTP integration | Persisted success, duplicate safety, failure rollback, policy and liveness cases | Four HTTP cases passed in the disposable harness; focused adapter + HTTP run 28.137 s | Pending |
| `python3 scripts/check_repository.py` and diff checks | Pass | Passed; 165 repository files checked for T2A | Passed |
| Independent security/dependency/concurrency verification | No reproducible candidate-caused blockers | Real-binary HTTP probes passed; lock timeout, two-slot admission, unchanged rejection/duplicate snapshots, rollback and baseline liveness verified. Audit: 191 dependencies, 1,378 advisories, no warnings | Passed |
| Full PostgreSQL infrastructure suite | All existing and registration cases pass | 38 tests passed, 757.663 s including adapter and HTTP targets | Passed |

## Risks, assumptions, and decisions

Risk items 2 (credentials), 4 (async hashing/duplicate races), 5 (new dependencies).
User approved Argon2id and the password policy. ADR-009/010 record the selected
parameters, offline common-password source and persistence failure boundary.
The final HTTP unit exceeds the advisory 400 authored-line target; endpoint,
public tests and contract remain together after one cohesive slicing pass.
Do not shrink tests/docs to fit; any oversized PR requires an explicit publication
size exception. Generated lock changes are counted separately;
stacked-to-main strategy approved. First slice implements credential primitives
only (ADR-009); registration endpoint own checks passed; independent verification and native review acknowledged. Generated Cargo.lock growth
is tracked separately from authored review lines; a publication-size exception
may be required for the inseparable lockfile. No PR is authorized yet.

## Completion notes

T1 local commit: `97961c7611afbdfb69d1e6187596e76f4a4d77a7`; native review acknowledged.
T2 persistence commit: `d1e108b563678760c22ad09c31eb10ccb1709293`; native review acknowledged.
HTTP registration is implemented locally; final 31 Cargo tests, focused database cases and full infrastructure suite (38 tests, 757.663 s before supplemental test-only regressions) passed. Independent verification and clean audit passed; final native four-lens review completed without findings and exact acknowledgement confirmed.
The ODD feature document is the task/specification logbook. Keep this
execution record active until verification and eventual merge are confirmed.

HTTP local commit: `406acbe5cad18b07206d3a800afae2a993842be0` (1,097 changed lines, including 15 generated lock lines). User granted this exact candidate review; all four lenses returned no findings and its exact acknowledgement confirmed completion. No push or PR performed.

All P04.02 implementation and verification work is complete locally. Login, sessions, email verification/recovery and Phase 04 completion remain out of scope. Publication requires separate approval and explicit size exceptions for oversized PRs.
