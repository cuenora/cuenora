# Execution plan: Isolated local PostgreSQL

- **Status:** Completed
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.06; archive merged P02.05
- **Tracking issue / PR:** [PR #7](https://github.com/cuenora/cuenora/pull/7) (merged)
- **Owner:** Codex

## Goal and user-visible outcome

Provide local Docker Compose PostgreSQL with persistent, project-scoped storage
and verify startup, password authentication, restart persistence, and isolation.
Archive the merged P02.05 execution plan.

## Existing state and evidence

- P02.05 merged via PR #6, both hosted CI checks passed.
- No Compose service or infrastructure test exists.
- Local Docker Engine 29.9.2 and Compose v5.6.1 are available.
- The backend remains dependency-free and has no database integration.

## In scope

- Local PostgreSQL Compose service, setup instructions, and executable smoke test.
- Archive verified P02.05 and mark P02.06 only after verification.

## Out of scope

- SQLx, application database roles, schema migrations, financial records, HTTP,
  authentication features, CI changes, example env files (P02.07), and deployment.

## Constraints and invariants

Use only synthetic test data. No embedded credentials or trust authentication.
Publish only on loopback; scope volumes and networks by Compose project. Tests
must use fresh project names and remove only their own resources. Preserve .atl.
Do not start the default development project during verification.

## Steps

- [x] 1. Inspect existing setup and archive verified, merged P02.05.
- [x] 2. Observe failing lifecycle tests before adding Compose configuration.
- [x] 3. Add minimal Compose configuration and documentation.
- [x] 4. Verify isolated lifecycle and independent review; record results.

## Acceptance criteria

- [x] Missing password prevents startup configuration.
- [x] PostgreSQL is healthy, accepts valid password and rejects an invalid one.
- [x] Only 127.0.0.1 is published, with an overridable host port.
- [x] Data survives container recreation; separate projects do not share data.
- [x] Test containers, networks and volumes are cleaned up after verification.
- [x] Setup documents start/check/connect/stop and explicit data removal.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 tests/infrastructure/test_postgres_compose.py` | Isolated lifecycle passes | Passed (exit 0) | Pass |
| `python3 scripts/check_repository.py` | Text conventions pass | Passed (exit 0) | Pass |
| `git diff --check` | Clean whitespace | Passed (exit 0) | Pass |

## Risks, assumptions, and decisions

- Risk: item 5 (new local database environment), item 2 (local credential and port policy).
- PostgreSQL 18 is supported. Use official `postgres:18-bookworm`, staying on
  major 18 while receiving image updates. Major upgrades require a separate plan.
- PostgreSQL 18 image storage mounts at `/var/lib/postgresql`.
- Require a user-provided password; the bootstrap role is for local administration,
  not a production application role. Application privileges are deferred.
- Keep default Compose name cuenora-local; `-p` selects isolated projects.
- No changes to shared Rust helpers or command contracts.

## Completion notes

RED observed: both new tests failed before the Compose file existed. The first
implementation then exposed loopback TCP trust: an incorrect password succeeded.
Adding `POSTGRES_INITDB_ARGS: --auth-host=scram-sha-256` fixed this; both tests
passed in 20.466 seconds, including cleanup assertions. Only disposable projects
were created. The missing-password test validates a nonzero configuration exit.
No prior Compose command exists at baseline for a differential comparison.

Repository validation and whitespace checks passed. Independent verification
passed both tests in 20.320 seconds from a fresh scratch copy with identical
Compose/test bytes; no blockers found. Its Docker resources were cleaned by the
test and the parent removed its scratch directory. The parent spot check passed
both tests in 20.569 seconds.

Native review `review-a27e16a9bf32438f` approved the risk, resilience, readability,
and reliability inspection. Its exact acknowledgement completed and consumed the
review authority. No blocking findings or code corrections were required.
Non-blocking advisory R4-001: cleanup currently stops at its first removal or
assertion failure, so a second temporary project might remain in that failure
case. A later improvement can attempt cleanup for all recorded projects before
reporting failures; this advisory did not reopen the approved review.

Implementation commit `da29dc7` was merged through
[PR #7](https://github.com/cuenora/cuenora/pull/7) as
`095de754e666b7a8e9a71ad8975ad3b860cde4ae`. Both hosted checks,
`Repository / Validate` and `Backend / Rust`, passed. The local `main` branch
was synchronized and the obsolete implementation branch removed after confirming
its tree matched the merge. This verified plan is now archived in `completed/`.
The non-blocking cleanup advisory above remains a follow-up. No P02.07 work was
started.
