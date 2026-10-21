# Execution plan: Rust dependency audit policy

- **Status:** Completed (merged and verified)
- **Date:** 2026-10-21
- **Roadmap task ID(s):** P02.10
- **Tracking issue / PR:** [PR #13](https://github.com/cuenora/cuenora/pull/13)
- **Owner:** Codex

## Goal and user-visible outcome

Retain existing Rust quality checks and audit the locked dependency graph against
RustSec on PRs, main pushes, manual runs, and a weekly schedule.

## Existing state and evidence

- Format, Clippy warnings-as-errors, and locked workspace tests already run in CI.
- No Rust dependency audit, audit configuration, or written audit policy exists.
- Dependabot currently updates GitHub Actions only.
- Compiler 1.99.1 is pinned; cargo-audit is not installed locally.
- The repository uses hosted Ubuntu jobs, read-only contents permission, and
  checkout without persisted credentials.

## In scope

- Pin cargo-audit 0.22.2 as a development tool and add an explicit lockfile audit.
- Document failure/warning/exception policy and local reproduction commands.
- Add a weekly schedule while preserving existing CI triggers/check names.
- Verify real lockfile and synthetic safe/vulnerable/error cases without mutations.

## Out of scope

- Branch protection/dependency-review settings (P02.11), ADR-001, runtime features,
  new Rust application dependencies, auto-fixes, issue creation, blanket ignores,
  licensing or binary vulnerability scanning, and unrelated cleanup findings.

## Constraints and invariants

Use the pinned compiler. Audit the existing Cargo.lock explicitly; never generate
or modify dependency resolution during scanning. No write-capable workflow token,
PR comments, or automatic issue creation. Fail installation/database/network errors
rather than reporting them as successful scans. Preserve .atl and existing checks.

## Steps

- [x] 1. Inspect current CI and upstream RustSec command semantics.
- [x] 2. Install the pinned tool outside the workspace and exercise audit policy.
- [x] 3. Add audit CI job and document scope, reproduction, and exception handling.
- [x] 4. Run checks and independent verification, then record actual results.

## Acceptance criteria

- [x] Existing format/Clippy/test commands and hosted/read-only CI protections remain.
- [x] Audit fails on known vulnerabilities and denied warning categories.
- [x] Missing/malformed lockfiles and unavailable advisory data fail nonzero.
- [x] Safe lockfile passes; denied warning fixture fails with policy flag.
- [x] Auditing leaves manifests and lockfiles unchanged.
- [x] CI and documentation use the same fixed audit version and command.
- [x] Scheduled and PR-time scanning are configured without changing branch rules.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pinned cargo-audit installation | Tool version 0.22.2 | Passed (observed) | Pass |
| Real `cargo audit --file Cargo.lock --deny warnings` | No blocking findings; resolution unchanged | Passed (observed) | Pass |
| Synthetic safe/vulnerable/denied-warning lockfiles | Exit 0 / nonzero / nonzero | Passed (observed) | Pass |
| Missing/malformed lockfile or advisory DB | Nonzero exit without file changes | Passed (observed) | Pass |
| Existing Cargo format, Clippy, locked tests/build | All pass | Passed (observed) | Pass |
| Workflow structural readback | Existing triggers/jobs preserved, weekly audit configured | Passed (observed) | Pass |
| `python3 scripts/check_repository.py` | Text conventions pass | Passed (observed) | Pass |
| `git diff --check` | Clean whitespace | Passed (observed) | Pass |

## Risks, assumptions, and decisions

- Risk: item 5 (CI and development tool installation), item 2 (audit failure policy).
- Use cargo-audit 0.22.2 with --locked and --no-default-features at installation:
  lockfile auditing needs no optional binary scanner or automatic-fix feature.
- `--file Cargo.lock` prevents missing-lockfile generation; scan never updates deps.
- `--deny warnings` blocks vulnerabilities plus supported unmaintained, unsound,
  yanked and tool-self advisory categories. Informational notices remain visible
  but are not universally denied by this flag. No severity threshold or ignores.
- Weekly schedule runs all existing CI jobs as well, avoiding conditional skips
  or duplicate workflows; fresh RustSec data is fetched for every real scan.
- Remove the pre-workspace success/skip placeholder now that the workspace exists;
  missing backend inputs must fail Rust checks.
- Fix/update affected dependencies rather than suppress findings. Any exceptional
  ignore requires maintainer approval, rationale, scope, expiry and tracked review;
  this change introduces no ignores or exception config.
- No meaningful product RED test applies to CI wiring. Verify the exact native
  audit command with deterministic negative fixtures instead of mirror tests.
- Sources: https://github.com/rustsec/rustsec/tree/cargo-audit/v0.22.2/cargo-audit
  and its upstream configuration and lockfile implementation.

## Completion notes

Installed cargo-audit 0.22.2 from its locked release without default features
using Rust 1.99.1 in an outside-workspace tool directory (2m50s build). Online
scan fetched 1,348 RustSec advisories and checked 42 locked dependencies with no
blocking findings (exit 0). Verification overrode only the advisory DB storage
path to a task-specific temporary directory; fetch and registry yank checks ran.

Synthetic fixtures passed: patched version exit 0; vulnerable version nonzero
with the exact fixture advisory; unmaintained default warning exit 0 and denied
warning nonzero. Missing/malformed lockfiles and missing advisory DB all failed
nonzero. Missing lockfile was not generated. Fixture and actual workspace hashes
confirmed manifests/lockfiles unchanged, including after rejected commands.

Pinned fmt, Clippy, locked workspace tests and build all passed. YAML parsed and
structural checks verified triggers, job names, hosted/read-only setup, command
versions, existing Rust checks, no silent workspace skip or continue-on-error.
Repository conventions and whitespace checks passed. Hosted CI passed all three
checks: `Repository / Validate`, `Backend / Rust`, and `Backend / Audit`.
Independent verification passed safe/vulnerable,
denied-warning and missing/malformed lockfile/database cases, plus a copied real
lockfile against the fetched advisory snapshot. It confirmed all hash invariants,
workflow structure, and unchanged application files, with no blockers. Parent
removed the independent scratch directory. A fresh online parent spot check also
passed after independent verification, with the same advisory/dependency counts.

Native review `review-20f1cef88907b18b` approved all four lenses with no findings.
Its exact acknowledgement completed and consumed the review authority; no code
corrections were required. Parent removed the task-specific tool installation,
build output, advisory database, and probe script after verification and review.
The installation command above remains the documented local prerequisite.

PR #13 merged on 2026-10-21 as `beea84c2ffea3f0749e18d68a4bceb0c7297a9d5`.
Local `main` was synchronized and the integrated feature branch was removed
after confirming its tree matched `main`. This plan is archived in `completed/`.
No P02.11 repository settings were changed.
