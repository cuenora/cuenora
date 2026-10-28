# Planning documents

This directory holds Cuenora's **version-controlled development plans**. `docs/plans/` is our repository convention; it is not a Git or GitHub-mandated path.

## Start here

- **[Master plan](master-plan.md)** — product scope, architecture, financial rules, 16-phase roadmap, 186 task IDs, and acceptance examples. It records intent, not automatically verified completion.
- **[Design guide](../../DESIGN.md)** — entry point for the visual direction, design-system rules, Pen source and review exports.
- **[Product requirements](../product/prd.md)** and **[product backlog](../product/backlog.md)** — the agreed MVP baseline and its story-to-task acceptance mapping. Follow the master plan's current dependency order; P15.06 is retained as a superseded slot for rate changes already included in MVP.
- **[Exact money specification](../product/money.md)** — approved amount policies, implemented domain representation, technical bounds and verified acceptance cases for P03.01–P03.02.
- **[Date and time-zone semantics](../product/dates-and-time-zones.md)** — P03.03 business dates, audit instants, profile/rule zones and acceptance examples; runtime implementation is pending.
- **[Identity and access strategy](../product/identity-and-access.md)** — P04.01 first-party credentials, open registration, email verification and technical prerequisites for P03.04.
- **[Identity storage](../architecture/identity-storage.md)** — P03.04 migrations, email keys, credential/session constraints and remaining application responsibilities.
- **Registration and initial login** — verified and merged execution records for [P04.02](completed/2026-10-25-registration.md) and [P04.03](completed/2026-10-26-secure-login.md); financial authorization and remaining Phase 04 work are not completed by these records.
- **Session rotation and revocation** — [P04.04 execution record](completed/2026-10-27-session-rotation.md), verified and merged through [#55](https://github.com/cuenora/cuenora/pull/55) into [#54](https://github.com/cuenora/cuenora/pull/54), then main (`ad3a1f5`). Financial ownership and logout/global endpoints remain pending.
- **Authenticated identity middleware** — [P04.05 completed execution record](completed/2026-10-27-authenticated-identity.md); reusable route-scoped validation with no new production endpoint. Implementation, independent verification and native review passed; [PR #57](https://github.com/cuenora/cuenora/pull/57) merged into `main` (`32afa03`).
- **Typed API errors and request IDs** — [P04.07 completed execution record](completed/2026-10-27-api-errors.md); implemented in `2340d2e` with own/independent verification and native review acknowledged; [PR #59](https://github.com/cuenora/cuenora/pull/59) squash-integrated into `main` (`3bb2d07`), with full published-tree equality verified locally. P04.06 remains deferred until financial resource endpoints exist.
- **Authentication throttling** — [P04.08 completed execution record](completed/2026-10-27-auth-throttling.md); bounded single-instance limits and explicit proxy trust verified locally in `d1bb428`; own/independent checks passed and native review acknowledged. [PR #61](https://github.com/cuenora/cuenora/pull/61) merged into `main` (`a62e02e`); full published-tree equality verified.
- **Token rejection and refresh reuse** — [P04.09 completed verification record](completed/2026-10-28-token-rejection-verification.md); 14 existing integration tests passed against disposable PostgreSQL on 2026-10-28, with no runtime changes. [PR #63](https://github.com/cuenora/cuenora/pull/63) merged into `main` (`bcc9e8e`); full published-tree equality verified.
- **Logout and global revocation** — [P04.11 execution record](active/2026-10-28-session-logout.md); implemented and independently verified locally, with commit/native review pending. No financial ownership or recovery workflows added.
- **[Account storage](../architecture/account-storage.md)** — P03.05 account fields, owner-scoped reference targets and the future application boundary.
- **[Category storage](../architecture/category-storage.md)** — P03.06 global built-ins, fixed identifiers and the read-only storage boundary.
- **[Transaction storage](../architecture/transaction-storage.md)** — P03.07 exact event amounts, lifecycle values, owned references and deferred posting responsibilities.
- **[Account movement storage](../architecture/account-movement-storage.md)** — P03.08 signed postings, ownership, append-only guards and deferred financial checks.
- **[Transaction audit storage](../architecture/transaction-audit-storage.md)** — P03.09 minimal append-only metadata and the deferred automatic auditing boundary.
- **[Financial storage constraints](../architecture/financial-constraints.md)** — P03.10 currency/account compatibility, savings-income guards and reference-preserving account changes.
- **[Verbal identity](../product/verbal-identity.md)** — voice principles, tone, and financial copy examples for P01.04.
- **[Navigation and primary flows](../product/navigation-and-flows.md)** — logical destinations, user journeys, and alternate paths for P01.05.
- **[Home wireframe](../product/home-wireframe.md)** — pen.dev sketches, state variants, and interaction notes for P01.06.
- **[Accounts and Transactions wireframes](../product/accounts-and-transactions-wireframes.md)** — lists, details, recording flows, and recovery specimens for P01.07.
- **[Recurrence, Savings, and Insights wireframes](../product/recurrence-savings-insights-wireframes.md)** — planned payments, interest distinctions, reports, and state specimens for P01.08.
- **[Initial color palette and typography](../product/color-and-typography.md)** — completed P01.09 blue palette, measured contrast, open-font specimens and three high-fidelity mobile references.
- **[Initial UI design system](../product/ui-design-system.md)** — completed P01.10 semantic tokens, reusable controls, interface/recovery states and adaptive specimens; see the [merged execution record](completed/2026-10-25-ui-design-system.md).
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
