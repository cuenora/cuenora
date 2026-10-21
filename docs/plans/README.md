# Planning documents

This directory holds Cuenora's **version-controlled development plans**. `docs/plans/` is our repository convention; it is not a Git or GitHub-mandated path.

## Start here

- **[Master plan](master-plan.md)** — product scope, architecture, financial rules, 16-phase roadmap, 186 task IDs, and acceptance examples. It records intent, not automatically verified completion.
- **[Plan template](TEMPLATE.md)** — start a focused plan for a nontrivial change or multi-step implementation.
- **`active/`** — plans for in-progress, scoped work.
- **`completed/`** — completed execution plans, kept for traceability.

## File naming

Use `YYYY-MM-DD-short-topic.md` for focused plans, e.g. `active/2026-10-21-bootstrap-rust-workspace.md`. Keep the master plan in its stable path.

## How agents should work with these files

1. Read root [`AGENTS.md`](../../AGENTS.md), this index, and the relevant section of the master plan.
2. Select the smallest unfinished roadmap task(s) and inspect the code; do not infer that an unchecked task is completely absent or that a checked one still passes.
3. For significant or cross-cutting work, copy `TEMPLATE.md` into `active/` and fill it out **before** changing implementation.
4. Define concrete acceptance criteria and verification commands, then implement in small steps.
5. Record outcomes, commands actually executed, design decisions, blockers, and known follow-ups. Do not invent pass results.
6. Once the corresponding PR is merged and acceptance criteria are met, move its plan to `completed/`, preserving its filename and evidence. Use GitHub issues/PRs to track live progress; do not treat unchecked roadmap items as GitHub issue statuses.
7. Modify the master plan only for intentional scope/decision changes or verified completion, not for speculative implementation notes.

## Priority and authority

User instructions and approved decisions take precedence. `AGENTS.md` gives execution constraints; the master plan is a proposed roadmap. Existing working code and tests must be inspected before making changes. If a plan conflicts with implementation or another document, raise it explicitly and record the resolution.

## Planning is not implementation

The presence of a task or a folder in this directory does not mean that the associated API, mobile app, database, or CI job already exists. Validate the actual repository tree first.
