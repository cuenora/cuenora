# Execution plan: Terminal password recovery

- **Status:** Completed
- **Date:** 2026-11-03
- **Roadmap:** P04.12 recovery runtime follow-up
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/106
- **Owner:** Maintainer / agent

## Goal and user-visible outcome

Request a recovery email, copy its one-time token into a terminal, and explicitly
save a new password. All old sessions are revoked; normal login is required.
The maintainer explicitly approved this backend/email/terminal-only boundary.
The specification and task evidence live in
[`odd/tasks/password-recovery-terminal.md`](../../../odd/tasks/password-recovery-terminal.md).

## Existing state and evidence

The approved protocol and ADR-022 define recovery semantics. Existing verification
provides bounded Resend delivery, registration provides password policy/Argon2id,
and login rechecks its locked credential snapshot. Pen board `B5taoG` supplies
the recovery message; a copyable token replaces its link/CTA because no client
landing page exists. No new provider credentials or dependency are needed.

## Scope and invariants

- Add purpose-bound digest-only recovery challenges and token-free reset notices.
- Anonymous generic request acceptance; strict input, separate limits and queue.
- Atomic reset, checked revision increment, all-session revocation and preserved
  email verification. No automatic login or financial mutation.
- Bounded notification delivery retries survive restart without storing secrets.
- Update contracts, terminal helper and deterministic isolated tests.
- No real email, remote execution, publication, mobile or web landing screen.

## Steps and acceptance

See T1–T4 and S1–S6 in the linked specification; no roadmap phase is closed by
this runtime slice alone. Rejected/expired/replayed tokens must leave data unchanged;
concurrent confirmation must have one winner; stale old-password login must fail.

## Verification

| Command or scenario | Expected | Observed |
| --- | --- | --- |
| Focused Rust HTTP/store/mail tests | Pass | Passed |
| Disposable PostgreSQL recovery/migration tests | Pass | Passed: store7, HTTP4, security6, preservation and fresh schema |
| Existing verification/login/session regressions | Pass | Passed in disposable PostgreSQL |
| Python terminal helper tests | Pass without live mail | Five recovery tests passed; all helper tests10 and explicit no-mail local bootstrap1 passed |
| Rust format, Clippy, workspace and repository checks | Pass | Passed |
| Independent security/concurrency verification | No change-caused blocker | Passed: six isolated probes plus differential auth/mail compatibility; scoped rolling-mailbox recheck passed |
| Human real-mail smoke test | Recovery + notification arrive | Maintainer-reported pass: reset/replay/old-password/old-refresh/new-login checks and separate notification receipt |

## Risks and delivery

Risk: items 1, 2, 3 and 4 (new persisted challenge/outbox schema, credentials,
shared adapter compatibility and transactional concurrency). Authored diff is
expected to exceed 400 lines; the maintainer explicitly approved one review unit with a size exception
and a local commit. No push or PR is authorized.

## Completion notes

Implementation and deterministic checks are complete locally. Independent probes
were retained as `password_recovery_security.rs`; they use a guarded test-only
16-connection pool so deliberately blocked operations leave an observer slot.
The known/unknown mailbox limiter uses rolling timestamps, preserving cooldown
at hourly boundaries (observed RED → GREEN regression).

Compatibility checks compare baseline/candidate auth status/body/cache and the
existing verification email payload. Native RDD is disabled by the global user
preference and was not enabled. The maintainer approved one local commit with a size exception and reported
successful real recovery-mail and password-change notification receipt. Publication
remains unauthorized; no production capacity claim is made. Pre-existing untracked `.atl/` and Python caches were preserved.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `56295cd feat(auth): add terminal and browser password recovery (#106)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
