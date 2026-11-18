# Execution plan: backend filters and maintained dates

- **Status:** Completed and merged
- **Date:** 2026-11-18
- **Roadmap task ID(s):** P05.12, P03.03 maintenance
- **Pull requests:** [#175](https://github.com/cuenora/cuenora/pull/175), [#176](https://github.com/cuenora/cuenora/pull/176)
- **Owner:** Maintainer-directed agent

## Goal and user-visible outcome
Add transaction kind and currency filters and eliminate confirmed unmaintained Chrono dependencies without changing existing endpoint contracts.

## Existing state and evidence
`transaction_listing.rs` shares a static predicate between cursor admission and page reads. Before this work, the query supported date/category/account filters only. RustSec advisories RUSTSEC-2026-0543 and RUSTSEC-2026-0555 confirm unmaintained dependencies with no patched version.

## In scope
Backend listing, maintained date library, compatible contracts, regression tests and documentation. The authoritative task log is [the ODD record](../../../odd/tasks/backend-filters-maintained-dates.md).

## Out of scope
Mobile UI changes, schema migrations, publishing, deployment and unrelated refactors.

## Constraints and invariants
Currency matches either transfer side once. Preserve ownership, pagination, strict dates, explicit IANA zones, month-end policies, exact financial calculations and endpoint response/error contracts. Bundle timezone data rather than introduce an OS tzdata prerequisite.

## Steps
- [x] Implement kind/currency filters with tests and contract documentation.
- [x] Replace Chrono with maintained date/time support and record the decision.
- [x] Verify pinned Rust, disposable PostgreSQL acceptance and cargo audit.

## Acceptance criteria
- [x] Both filter semantics compose with existing predicates and cursors.
- [x] All previous date/zone behavior remains compatible.
- [x] No Chrono packages remain in the dependency graph; audit denies warnings successfully.

## Verification
Rust domain/API tests, full serial workspace suite, Clippy, format, audit and repository checks passed. Disposable PostgreSQL listing, profile, opening, savings and planning passed; recurring acceptance passed after correction. Independent S1–S3 verification passed; see ODD log for ordering and transient healthcheck failures.

## Risks, assumptions, and decisions
Dependency and shared date-helper migration requires independent verification. Independent verification passed; the maintainer approved a size exception for the date migration.

## Completion notes
Implementation and bounded correction verified. Two local review units approved, with a size exception for the migration. Both PRs are integrated in main (#175: `dd978be`; #176: `4f999db`).
