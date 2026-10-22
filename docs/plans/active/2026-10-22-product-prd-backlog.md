# Execution plan: Initial PRD and backlog

- **Status:** Active
- **Date:** 2026-10-22
- **Roadmap task ID(s):** P00.10
- **Tracking issue / PR:** Not created
- **Owner:** Maintainer and agent

## Goal and user-visible outcome

Consolidate the agreed product definition into PRD v0.1 and a prioritized backlog
linked to existing stories, numerical examples, and roadmap tasks.

## Existing state and evidence

P00.02–P00.09 documents define the audience, stories, optional accounts, manual
cross-currency transfers, savings restrictions, glossary, and numerical examples.
The release table still places effective-dated rates in v1 despite P00.07.

## In scope

- PRD, backlog, document navigation, and consistent release classification.

## Out of scope

- Financial implementation, new features, commits, publishing, or PR creation.

## Constraints and invariants

Preserve accepted decisions; identify unresolved calculation policies explicitly.
Documentation completion must not imply product functionality or user validation.

## Steps

- [x] Write PRD v0.1 and backlog using existing task IDs and evidence.
- [x] Reconcile release table and link P00.10 deliverables.
- [x] Verify document structure, traceability, links, and repository checks.

## Acceptance criteria

- [x] All ten stories and examples are traceable through the backlog.
- [x] MVP, exclusions, acceptance criteria, and open implementation questions are explicit.
- [x] Roadmap completion reflects documents, not implementation.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| `python3 scripts/check_repository.py` | Pass | Passed | Passed |
| `git diff --check` | Pass | Passed | Passed |
| Structural/link/traceability checks | Pass | Passed | Passed |

## Risks, assumptions, and decisions

The maintainer is the only current user. No external research or bank terms are
claimed. Phase 00 defines the product; financial implementation remains pending.

## Completion notes

Created PRD v0.1 with ten requirement IDs and a value-ordered backlog mapping
all ten stories to roadmap tasks and acceptance examples. Reconciled
effective-dated interest rates as MVP scope. Required specification gaps remain
explicit. Repository validator (54 Git-visible files), whitespace, relative links,
roadmap-ID references, and ten-story/ten-requirement checks passed. No financial
application tests were run: this is documentation only. No commit or PR created.
Keep active until merge.
