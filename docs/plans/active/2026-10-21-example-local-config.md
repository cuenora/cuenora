# Execution plan: Example local configuration

- **Status:** Active (verified; awaiting PR and merge)
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.07
- **Tracking issue / PR:** Not created
- **Owner:** Codex

## Goal and user-visible outcome

Provide a secret-free local environment example for the existing PostgreSQL
Compose service, with explicit instructions for a private working copy.

## Existing state and evidence

- P02.06 and its archival are merged; Compose already requires a password.
- Compose reads CUENORA_POSTGRES_PASSWORD and optional CUENORA_POSTGRES_PORT.
- `.gitignore` already ignores `.env` and permits `.env.example`.
- Setup currently documents shell-exported credentials; no example file exists.
- No backend configuration parser or database client exists.

## In scope

- Root `.env.example` with an empty password and the existing port default.
- Instructions for copying, filling, protecting, and explicitly loading `.env`.
- Verify both rejected empty and accepted populated examples in temporary state.

## Out of scope

- Backend config parsing, new variables, credentials, runtime Compose changes,
  database startup, CI changes, task runners, R4-001 cleanup improvement, P02.08.

## Constraints and invariants

No actual secrets in version control or tool output. Do not create or overwrite
the user's `.env`. Preserve existing shell-based setup. Use isolated temporary
files for verification; no database or container is needed. Preserve `.atl/`.

## Steps

- [x] 1. Inspect existing Compose variables, ignore rules, and setup.
- [x] 2. Add the example and document explicit file loading.
- [x] 3. Verify configuration and Git ignore behavior; mark only P02.07.

## Acceptance criteria

- [x] Example contains only existing variables, with a deliberately empty password.
- [x] Unedited example fails Compose validation without an exported password.
- [x] Temporary populated copy validates and preserves expected port/credential wiring.
- [x] Shell exports keep their existing behavior and precedence is documented.
- [x] `.env` remains ignored while `.env.example` can be committed.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Compose config with unedited example, sanitized environment | Nonzero exit, missing-password diagnostic | Passed | Pass |
| Compose config with temporary populated copy, sanitized environment | Exit 0 and existing PostgreSQL configuration | Passed | Pass |
| Original shell variables with no env file | Existing behavior unchanged | Passed | Pass |
| Git ignore checks | Ignore .env; permit .env.example | Passed | Pass |
| `python3 scripts/check_repository.py` | Text conventions pass | Passed | Pass |
| `git diff --check` | Clean whitespace | Passed | Pass |

## Risks, assumptions, and decisions

- Risk: none. Example and documentation only; no runtime auth/config policy changes.
- Keep the password empty to prevent a reusable default credential. Document
  generating a private password for a new volume and reusing it for existing data.
- Explicit `--env-file .env` avoids assumptions about Compose file discovery.
- Shell variables override env-file entries, matching existing Compose behavior.
- Passive example/documentation change: no meaningful code RED test; use direct
  Compose validation in isolated temporary files rather than adding mirror tests.
- No Rust checks or database lifecycle rerun needed: their implementation is unchanged.

## Completion notes

All scenarios passed using Docker Compose v5.6.1 in temporary files outside the
repository. The documented copy command created mode 0600, refused a second copy,
and preserved the existing file hash on rejection. Unedited config returned a
nonzero exit with the required-password diagnostic. Populated config exited zero
and retained loopback binding, default port, and expected credential mapping.
Original shell-only configuration matched the env-file model, and shell port
overrides took precedence as documented. Runtime Compose bytes match the base.
Git ignore checks confirmed `.env` excluded and `.env.example` permitted.
Repository validation and whitespace checks passed. No password was printed, no
container started, no workspace `.env` created, and temporary files were removed.

Native assessment reported medium (example configuration), with
`review_due: false` and reason `under_budget`; no native review was started.

Marked only P02.07 after verification. Deliver the verified change on
`docs/example-local-config` through the commit and PR workflow. Keep this plan
active until merge. P02.08 and the prior cleanup advisory are not started.
