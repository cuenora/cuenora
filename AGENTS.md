# Agent instructions — Cuenora

These instructions apply throughout this repository. Read them before proposing or editing code. More specific `AGENTS.md` files may be added later for subprojects; do not create them until a real need exists.

## Project identity and current status

- **Cuenora** is a personal, non-commercial, MIT-licensed, open-source personal-finance project.
- Intended product: multiple financial accounts, dated income and expenses (including entries without an associated account), internal transfers, recurring expenses, interest-bearing savings, and clear financial statistics.
- Intended architecture: one monorepo with a Rust/Axum API, PostgreSQL/SQLx, shared Kotlin Multiplatform mobile logic, Jetpack Compose for Android, SwiftUI for iOS, and Maestro for shared mobile E2E flows.
- Current foundations: Rust domain money/accounts, PostgreSQL financial storage and authentication/session APIs. Financial resource endpoints, verification/recovery runtime and mobile applications remain pending; see the [delivery snapshot](docs/plans/master-plan.md#current-delivery-snapshot).
- **Do not mistake plans for implemented code.** Inspect the repository before choosing dependencies, assuming directories exist, or running commands.
- Work in **English**: source code, comments, documentation, issues, PR descriptions, and commits.

## Read-first documentation

1. [`README.md`](README.md) and [`docs/README.md`](docs/README.md) — public vision, documentation map and current-status entry point.
2. [`docs/plans/README.md`](docs/plans/README.md) — how implementation plans are organized and maintained.
3. [`docs/plans/master-plan.md`](docs/plans/master-plan.md) — long-term scope, financial invariants, roadmap task IDs, acceptance cases.
4. [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`SECURITY.md`](SECURITY.md) — contributor and vulnerability reporting policies.
5. [`docs/development/SETUP.md`](docs/development/SETUP.md) — verified setup and CI instructions.
6. Relevant architecture decision records, code, and tests for the task being worked on.

If documents disagree with existing implementation, investigate and flag the inconsistency; do not silently rewrite requirements or assume the code is correct.

## Working agreement

- Take the smallest useful step. Prefer one focused roadmap task (for example, `P03.01`) per PR, with tests and documentation.
- Before a substantial change, read the applicable roadmap phase. For cross-cutting work, create a focused execution plan from [`docs/plans/TEMPLATE.md`](docs/plans/TEMPLATE.md) in `docs/plans/active/`.
- Do not start unrelated features, large refactors, major dependencies, migrations, or architectural changes just because they are mentioned in the roadmap.
- Confirm assumptions in code and documented decisions. Record meaningful new architectural choices in an ADR (under `docs/architecture/decisions/`).
- Preserve existing user changes. Do not overwrite work, rewrite history, or directly push to `main`. Open a branch and PR; use the project's existing CI and branch rules.
- Update documentation when behavior or commands change. Mark a master-plan checkbox complete only when implementation and its acceptance criteria have been verified; never bulk-check planned work.
- Never claim tests passed unless they were actually run. Report what was tested and what could not be tested.

## Financial correctness — non-negotiable

- Never use binary floating-point values (`f32`/`f64`, Kotlin `Float`/`Double`, Swift `Float`/`Double`, or database `REAL`) for monetary amounts or financial calculations. Use exact decimal or integer-minor-unit representations with explicit currency, scale, and rounding rules.
- Do not mix currencies in totals without an explicit, dated exchange-rate policy. The MVP supports SEK, EUR, and USD, including manually recorded cross-currency transfers with explicit sent and received amounts and an optional exchange rate; see `docs/product/currencies-and-transfers.md`.
- MVP accounts are `Normal` or `Savings`; reuse the [account/name policy](docs/product/accounts.md). Ordinary income cannot target savings, including edits/reassignment; savings funding after opening uses transfers or actual credited interest.
- Categories are the [global read-only catalog](docs/architecture/category-storage.md), not user-editable records.
- An income or expense **without an account** contributes to income/expense reports but must **not** affect an account balance.
- Account balances derive from posted account movements, not client-provided balances. Opening balances are opening movements, not ordinary income.
- A transfer between owned accounts is **one business operation** with **two opposite account movements**. It must be atomic and must not count as income or spending. Implement safe idempotency for retried write requests.
- Recurring occurrences and forecast interest remain **planned**, with no effect on balances or realized income/expenses until explicitly confirmed/posted.
- A credited interest payment is distinct from projected interest. Make the interest basis, nominal annual rate, payout schedule, capitalization, and rounding explicit before implementation.
- Revisions, reversals, and voids preserve append-only movements through compensating/new postings. The [minimal audit](docs/architecture/decisions/007-minimal-transaction-audit.md) stores changed field names, not value snapshots; event, posting and audit changes must be atomic.
- Keep business event dates distinct from UTC audit timestamps; recurring schedules must have explicit time-zone and end-of-month policies.
- Make financial logic deterministic; keep core calculation code free of HTTP, database, system clock, and UI dependencies wherever practical.

## Security and privacy

- Treat all user and financial data as sensitive. Do not commit or print credentials, real account details, private keys, tokens, or financial records in logs, fixtures, or screenshots.
- Every user-owned resource read/write and account relationship must enforce authenticated ownership on the server. Never trust a client-submitted `user_id` or balance. Shared built-in categories are read-only reference data, never a reason to relax financial ownership checks.
- Authentication alone grants no financial authority: enforce verified email before account creation/posting, and a confirmed profile IANA zone when an operation needs a financial date. See the [identity](docs/product/identity-and-access.md) and [date](docs/product/dates-and-time-zones.md) policies; their pending runtime is not permission to bypass them.
- Validate all input; use parameterized SQL, least-privilege credentials, safe error messages, bounded queries, and appropriate rate limits.
- Protect sessions and refresh tokens; use platform-native secure storage where available.
- Never post security vulnerabilities in public issues. Follow `SECURITY.md` if present; otherwise use GitHub's private vulnerability reporting when enabled.
- Test authorization boundaries, failed transactions, retries, edge cases, and cancellation of financial operations.

## Architecture boundaries

- **Backend:** Prefer a modular monolith. Rust domain rules must not depend on Axum or SQLx. Transport, application use cases, domain, and persistence remain conceptually separate even if they initially live in only two crates.
- **API:** Keep the OpenAPI contract, JSON examples, and clients synchronized. Represent monetary amounts in API JSON as decimal **strings** with currency identifiers; do not expose them as floating-point JSON numbers.
- **Mobile:** Share reusable data/validation/client logic and `commonTest` through Kotlin Multiplatform. Keep Android UI in Jetpack Compose and iOS UI in SwiftUI; do not force UI parity at the expense of native behavior.
- **End-to-end:** Use shared Maestro flows when they represent the same behavior; add platform-specific steps only as needed. Platform integrations still need their own tests.
- **Database:** Schema changes require migrations, constraints, and integration tests against real isolated PostgreSQL using the existing infrastructure harness.

## Commands: run only when their prerequisites exist

From the repository root:

```bash
python3 scripts/check_repository.py
```

From the existing `backend/` workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

- Distinguish ordinary Cargo results from ignored PostgreSQL tests; use the isolated harness documented in SETUP.md when database behavior changes.
- Follow `backend/rust-toolchain.toml` for the active compiler version; do not silently switch the project toolchain.
- Run PostgreSQL integration tests only with an isolated test database. Do not point tests at production or personal data.
- Android/iOS toolchains, commands, and CI targets should be documented when those projects actually exist. iOS compilation requires macOS/Xcode.
- Do not weaken CI, skip test failures, or suppress Clippy/linter warnings merely to get a green check.

## Expected completion report

For each task, communicate: (1) what changed and which roadmap IDs it addresses, (2) which tests/commands were run and their outcomes, (3) any remaining assumptions or risks, and (4) follow-up work. Keep PRs reviewable and avoid declaring a phase complete without its exit criteria.
