# Cuenora — Master Project Plan

> **Status:** Planning baseline; not a statement of completed implementation.
> **Version:** 0.3 — 2026-10-22; aligned with PRD v0.1 and accepted Phase 00 decisions.
> **Origin:** English adaptation of the original 2026-10-21 personal-finance master plan; original task IDs (`P00.01`–`P15.08`) are preserved.
> **Project:** [cuenora/cuenora](https://github.com/cuenora/cuenora).
> **Purpose:** Build an open-source, non-commercial personal-finance app for Android and iOS with a Rust API, PostgreSQL, native user interfaces, and shared mobile logic and test cases.

**Reading note:** The roadmap is a proposed sequence. A checkbox is unchecked until the relevant result has been verified and recorded; several repository-setup actions may already have been performed outside this document. Verify current reality before proceeding. All architecture and schema details are proposals until implemented or approved in an ADR.

The [PRD v0.1](../product/prd.md), [backlog](../product/backlog.md), and linked
Phase 00 decisions define the current product baseline. The initial account
assumptions below remain subject to validation; checked product tasks document
decisions, not implemented financial behavior. The 186 original task IDs are
retained; P15.06 is a superseded slot, not an additional implementation task.

## 0. Vision and boundaries

### Product vision

Help individuals record, understand, and organize accounts, income, expenses, transfers, recurring payments, interest-bearing savings, goals, and financial trends.

### Non-negotiable principles

1. **Privacy:** A user must never access another user's financial records. Avoid sensitive amounts and identifiers in logs, traces, analytics, test artifacts, and screenshots.
2. **Correctness:** No binary floating-point arithmetic for money or financial interest.
3. **Traceability:** Every change to a posted account balance is attributable to a financial movement.
4. **Determinism:** The same inputs and rules yield the same balance and reports. Time zones, dates, and rounding are explicit.
5. **Server authority:** Clients can display estimates; the API validates and authorizes balance-changing operations.
6. **Incremental delivery:** Every small task is reviewable and testable without waiting for the entire product.
7. **Native mobile UX:** Jetpack Compose on Android; SwiftUI on iOS; shared Kotlin Multiplatform logic and unit tests.
8. **Simplicity:** Modular monolith before microservices; online-first MVP before offline synchronization.

### Product baseline and remaining assumptions

- One user owns multiple accounts: cash, checking, savings, and potentially credit accounts.
- Income and expenses may exist **without an assigned account**. They appear in reports but do not change account balances. This decision is [accepted under P00.05](../product/accountless-entries.md).
- MVP currencies are SEK, EUR, and USD. Transfers between different, user-owned accounts may use the same or different supported currencies, with recorded sent/received amounts and an optional rate. See the [accepted P00.06 decision](../product/currencies-and-transfers.md).
- The opening balance is a specific opening movement, **not** ordinary income.
- Forecast interest is not credited income until an actual interest payment is confirmed.
- Core features require authentication and persistence in PostgreSQL.
- Initial language choices may include English, Spanish, and Swedish; internationalization is planned, not assumed implemented.

## 1. Release scope

| Release | Included | Excluded initially |
| --- | --- | --- |
| MVP | Authentication; accounts and opening balances; dated and categorized income/expenses with optional account; same-currency and manually recorded cross-currency transfers; recurring expenses; interest-bearing savings with effective-dated rates and payout schedules; basic analytics and per-account payment-funding comparisons; security, tests, and backups | Bank connections, receipt OCR, AI, automatic FX, shared finances, real-money investments |
| v1 | Budgets, savings goals, reminders, CSV/JSON export, advanced filters, improved UX, localization | Complex offline synchronization and actual payment execution |
| v2+ | Offline-first; CSV/OFX import; attachments; shared accounts; automatic FX and exchange-rate history; reconciliation; automatic categorization; region-specific open banking | Unvalidated features and features requiring unsupported regulatory assumptions |

### Representative MVP user stories

- Create a SEK checking account with an opening balance and see its actual balance.
- Add a backdated grocery expense without assigning an account; see it included in that month's spending.
- Later link that expense to an account and update the balance **exactly once**.
- Transfer 500 SEK between two owned accounts without increasing reported income or expenses.
- Schedule monthly rent and confirm individual payments without deducting future payments early.
- Configure a savings account's nominal annual interest rate and payout cadence; see a forecast without counting it as received money.
- See dated income, spending, monthly net flow, and category breakdown, including accountless entries.

## 2. Proposed technology and architecture

### Backend

- Rust stable toolchain (version pinned by `backend/rust-toolchain.toml`), Tokio, Axum, Serde, tower-http, tracing.
- PostgreSQL with SQLx and versioned database migrations.
- Exact fixed-scale recorded money using signed integer minor units, per [ADR-002](../architecture/decisions/002-exact-money.md). Fractional calculation engines may use exact decimal or rational arithmetic; choose database types and constraints before migrations. See the [money specification](../product/money.md) for implemented scale, bounds and rounding.
- Versioned OpenAPI contract; use typed, compatible JSON with financial amounts as **decimal strings**, never JSON floats.
- Scheduled jobs may initially run inside the modular monolith, with a separate worker only when justified. Jobs and write requests must be idempotent.

### Mobile

- `mobile/shared` (final path to be confirmed): Kotlin Multiplatform `commonMain` and `commonTest`, Ktor HTTP, serialization, shared validation, models, and presentation/use-case logic where beneficial.
- `mobile/androidApp`: Kotlin + Jetpack Compose, Android Keystore for secrets.
- `mobile/iosApp`: Swift + SwiftUI, iOS Keychain for secrets.
- Online-first MVP with loading, error, and refresh states; a durable offline write queue and merge/conflict resolution are deferred.

### Testing strategy

- **Rust unit:** money, dates, interest, recurrence, domain invariants.
- **Rust integration:** isolated real PostgreSQL, migrations, persistence, transactional writes, HTTP behavior, cross-user access denial.
- **KMP common unit:** business-independent client models, parsing, validation, state, shared API logic for both targets.
- **Shared contract fixtures:** same JSON input/output expectations verified independently by Rust and Kotlin test runners.
- **Android platform:** JUnit, Compose UI/instrumented checks, Keystore interactions.
- **iOS platform:** XCTest/XCUITest, Keychain and SwiftUI integration.
- **Maestro E2E:** shared YAML flows wherever possible; platform-specific subflows when necessary.
- **CI:** Linux for Rust and Android; macOS/Xcode for iOS and its simulator when implemented.

## 3. Financial rules

### Monetary amounts and dates

- Model recorded money as `{ signed_minor_units, ISO_4217_currency }`, with exact decimal-string interchange. The [money specification](../product/money.md) defines implemented and tested precision, display scale, rounding and overflow policy.
- Input amounts are positive when registering income/expenses. Account movements carry the effect's sign.
- Recorded expenses and transfers may leave negative balances; insufficient recorded funds alone do not block them. Apply technical bounds without commercial amount caps.
- `occurred_on DATE` is the user-selected economic date; `created_at` / `updated_at TIMESTAMPTZ` are audit instants.
- Store an IANA user time zone for recurring schedules and calendar-based summaries.
- See [date semantics](../product/dates-and-time-zones.md) and [ADR-003](../architecture/decisions/003-date-semantics.md): profile zones are confirmed and stable; existing rules retain independent zones; realized dates cannot be future-dated at creation or explicit date edit. The definition is complete; runtime validation remains pending.
- Calculations may retain greater precision than final postings; document rounding at settlement.

### Accounts and accountless entries

- Accounts have owner, type, name, currency, and active/archived status.
- Ordinary income may be assigned to normal accounts, not savings accounts; enforce this on creation, edits, and later linking of accountless entries.
- Accountless income or expense contributes to reports, never the aggregate of account balances.
- Linking, unlinking, or editing a posted transaction creates/replaces/reverses its account movements atomically and **exactly once**.
- Do not sum balances across different currencies without a dated FX policy. MVP shows separate currency totals.

### Transfers

- A transfer is **one business event** with **two signed movements**: the amount sent in the source currency and the amount received in the destination currency. Amounts are equal only for same-currency transfers; it is not normal income or expense.
- Post both movements in one database transaction. If either write fails, neither balance changes.
- Reject same-account transfers, cross-user transfers, and unsupported currencies. Cross-currency transfers require both actual amounts; a user-entered rate is optional and does not override them.
- Use stable idempotency keys to protect against retries and duplicate submissions.
- If transfer fees are ever supported, represent them separately as expenses.

### Recurring payments

- A recurrence rule defines amount, currency, category, optional account, frequency, start and optional end, time zone, next due date, and end-of-month behavior.
- A `recurring_occurrence` starts as planned. Posting occurs only when that occurrence is confirmed; automatic posting is out of MVP scope by default.
- Enforce uniqueness such as `(rule_id, due_date)` and idempotent confirmation/generation. Pausing does not erase history.
- Initial cadences: weekly, monthly, quarterly, annually. Test month ends, leap days, time zones, and year transitions.

### Interest-bearing savings

- Savings means balances held in accounts designated for savings, with or without interest; do not duplicate these balances or earmark portions of everyday accounts. See the [accepted P00.07 definition](../product/savings.md).
- Savings with interest is an **account with additional terms**, not a separate, duplicated balance.
- A savings account may have an opening balance at creation (not income). Subsequent contributions must be transfers; the only direct income allowed is actual credited interest. Reject ordinary-income creation or reassignment into savings without changing records or balances. See the [funding rules](../product/savings.md#funding-and-allowed-income).
- The MVP stores a clearly identified **nominal annual rate**, not a silently inferred APY/EAR/TAE.
- Terms include effective dates, payout frequency and next payout, day-count basis (such as ACT/365), capitalization (simple or compound), settlement rounding, and optional user-entered withholding tax.
- Rates retain effective-dated history. Payout frequency supports daily, monthly, quarterly, and annual schedules independently of the annual rate quotation. Do not calculate forecasts from unidentified nominal/effective rate terms.
- Show **forecast interest** separately from **posted/credited interest**. Forecasts never change spendable balances.
- An actual interest payout becomes an `interest_income` transaction and a corresponding movement in the account that actually receives it; never also credit another account.
- Test zero rates; rate changes; deposits/withdrawals mid-period; short/long months; leap years; delayed payments; and corrections.
- Explain that forecasts may differ from bank statements depending on the bank's real conditions.

### Corrections, voids, and retention

- All financial records have an owner and auditable timestamps/changes.
- Edit, void, or reverse posted entries consistently and transactionally; never leave orphaned or doubled account movements.
- Archive accounts with history rather than silently deleting them. Data-erasure requests are a separate privacy workflow governed by applicable retention requirements.

## 4. Proposed data model

| Table | Responsibility |
| --- | --- |
| `users` | Identity, normalized email, locale and time-zone preferences, signup time |
| `auth_credentials`, `auth_sessions` | Credentials/password hash when applicable, session and refresh-token lifecycle |
| `accounts` | User-owned account type, name, ISO currency, active/archived status |
| `categories` | Owned or built-in income/expense classifications |
| `transactions` | Owned business operation, type, economic date, decimal amount and currency (both sent and received for cross-currency transfers), optional quoted transfer rate with explicit currency direction, category, description, state, idempotency metadata |
| `account_movements` | Signed, traceable account postings; **source of truth for account balances** |
| `transaction_audit` | Relevant actor, action, record, timestamp, safe change summary |
| `recurring_rules` | Schedule, amount, type, category, optional account, time zone, state |
| `recurring_occurrences` | Due date, planned/confirmed/skipped state, linked transaction, uniqueness |
| `savings_terms` | Account interest rules, day count, payout cadence, capitalization |
| `savings_rate_periods` | Nominal annual interest rate and validity interval |
| `idempotency_records` | User/operation/key, request fingerprint, prior result, expiration |
| `user_preferences` | Display currency as appropriate, locale, time zone, formatting |

**Schema guardrails:** choose suitable UUID keys; strong foreign keys; constraints on amount/type/currency; indices scoped by user, account, and economic date; ownership checks on relationships as well as top-level records; zero reliance on user IDs supplied by the client. Make migrations reproducible and rollback plans explicit.

**Separation of truths:** `transactions` records financial business events; `account_movements` records actual account effects. Financial statistics come from eligible posted business events; account balances from account movements. Planned recurring occurrences never enter realized figures.

### Invariants required in tests

1. `balance(account) = SUM(posted account_movements.signed_amount)`; opening balances are opening movements.
2. Account-linked income creates one positive posting; account-linked expense creates one negative posting.
3. Accountless income/expense creates **zero** account postings, but contributes to reports.
4. A transfer creates exactly two postings with opposite signs in their account currencies. Same-currency postings sum to zero; cross-currency postings use the recorded sent and received amounts and cannot be added across currencies.
5. No movement can point to an account belonging to a different user.
6. An idempotent request never posts a second balance change when retried.
7. Plans and interest forecasts create zero account postings until confirmed.
8. Edits and voids always preserve consistent balances.
9. Amounts in different currencies cannot be implicitly combined.
10. Reports exclude transfers, opening balances, and voided operations from ordinary income and expenses.
11. Savings rejects ordinary income on creation, edits, and reassignment; opening movements, transfers, and credited interest retain their distinct meanings.
12. Planning differences never modify posted balances; confirmed occurrences leave pending-payment totals exactly once.

## 5. Initial API contract (proposed)

Version prefix: `/v1`. UTF-8 JSON, typed errors (`code`, `message`, `details`, `request_id`), pagination, ISO dates, and money as a decimal string with ISO currency.

| Area | Illustrative endpoints |
| --- | --- |
| Auth | `POST /v1/auth/register`, `/login`, `/refresh`, `/logout`, `/logout-all` |
| Profile | `GET/PATCH /v1/me`, data export, authenticated account deletion |
| Accounts | `GET/POST /v1/accounts`, `GET/PATCH /v1/accounts/{id}`, archive |
| Categories | `GET/POST /v1/categories`, `PATCH /v1/categories/{id}` |
| Transactions | `GET/POST /v1/transactions`, `GET/PATCH /v1/transactions/{id}`, void |
| Transfers | `POST /v1/transfers`: source/destination, business date, sent/received amounts and currencies, optional rate quoted as destination units per source unit |
| Recurrence | `GET/POST /v1/recurring-rules`, occurrences, confirm/skip |
| Savings | Savings terms, forecast, actual interest payments |
| Statistics | Overview, cash flow, by category, per-account pending payments and planning difference through a selected date |
| Health | `GET /health/live`, `GET /health/ready` (minimal sensitive detail) |

Define HTTP status/error semantics, input bounds, permission requirements, sensitive POST `Idempotency-Key` behavior, and OpenAPI compatibility before building separate mobile API clients.

## 6. UX and accessibility

### Proposed MVP screens

- Short onboarding → sign in/register → create first account → home dashboard.
- **Home:** balances grouped by account/currency, income, expenses, net flow, next recurring payments.
- **Transactions:** timeline, filters, create income/expense/transfer, details.
- **Accounts:** list, detail, balance, transactions, create/edit/archive.
- **Planning:** recurring expenses, next due dates, posting status.
- **Insights:** monthly cash-flow charts, categories, date ranges, comparisons.
- **Savings:** interest terms, forecast, confirmed interest receipts.
- **Profile/Settings:** privacy, export, language, time zone, security, sign out.

### Interface principles

- Framework-independent design tokens for color, spacing, typography, radius, and icons.
- Native navigation and controls rather than forced visual uniformity.
- Loading, empty, offline, error, success, and validation states.
- Dynamic text sizing, VoiceOver/TalkBack, contrast, semantic labels, locale-aware dates and currency formatting.
- Matching stable accessibility IDs for shared Maestro test flows across platforms.

## 7. Executable roadmap — 186 tasks across 16 phases

Each task should fit a focused, testable change. A PR may combine tightly coupled tasks. Do not declare a phase finished before its exit criteria are met. Use GitHub issues/PRs and, for complex work, `docs/plans/active/` execution plans.

### Phase 00 — Product definition and decisions (no code)

- [x] P00.01 Write a one-sentence product mission.
- [x] P00.02 Define the primary target user and their problem. See the [target-user definition](../product/target-user.md).
- [x] P00.03 Write ten prioritized user stories. See the [prioritized stories](../product/user-stories.md).
- [x] P00.04 Define MVP inclusion and exclusions.
- [x] P00.05 Record the decision on accountless income and expenses. See the [accepted decision and examples](../product/accountless-entries.md).
- [x] P00.06 Record the MVP currency and transfer decisions. See the [accepted decision and examples](../product/currencies-and-transfers.md).
- [x] P00.07 Define savings representation and accounting. See the [accepted definition and examples](../product/savings.md).
- [x] P00.08 Create a glossary: account, transaction, movement, balance, due date, forecast and credited interest. See the [financial glossary](../product/glossary.md).
- [x] P00.09 Define ten numerical worked examples with expected results. See the [ten worked examples](../product/worked-examples.md).
- [x] P00.10 Write the initial PRD at `docs/product/prd.md` and the backlog. See [PRD v0.1](../product/prd.md) and the [initial backlog](../product/backlog.md).

**Exit:** PRD v0.1, glossary, defined MVP scope, and approved invariants.

### Phase 01 — Naming, visual identity, and early UX

- [x] P01.01 Prepare a name shortlist with legibility and availability criteria.
- [x] P01.02 Check domains, trademarks, and app-store identifiers for the finalist.
- [x] P01.03 Select a provisional or final name without blocking development.
- [x] P01.04 Define a clear, approachable, trustworthy, non-banking verbal identity. See the [verbal identity guide](../product/verbal-identity.md).
- [x] P01.05 Create the sitemap and primary user flows. See the [navigation and flow proposal](../product/navigation-and-flows.md).
- [x] P01.06 Sketch the Home wireframe. See the [Home sketch and state variants](../product/home-wireframe.md).
- [x] P01.07 Sketch Accounts and Transactions wireframes. See the [Accounts and Transactions sketches](../product/accounts-and-transactions-wireframes.md).
- [x] P01.08 Sketch Recurrence, Savings, and Insights wireframes. See the [Recurrence, Savings, and Insights sketches](../product/recurrence-savings-insights-wireframes.md).
- [ ] P01.09 Explore an accessible initial color palette and typography.
- [ ] P01.10 Define UI design tokens, components, and interface states.

**Exit:** User flows and low-fidelity screens that guide implementation.

### Phase 02 — Repository, tooling, and initial CI

- [x] P02.01 Create a Git monorepo and `README.md`.
- [x] P02.02 Select a license and contribution policy for open-source work.
- [x] P02.03 Create `backend`, `mobile`, `contracts`, `tests`, `infra`, and `docs` directories as needed.
- [x] P02.04 Configure a Rust 2024 Cargo workspace with `domain` and an `api` binary.
- [x] P02.05 Set up Rustfmt, Clippy, and task runner commands (`make` or `just`) if justified.
- [x] P02.06 Add local Docker Compose with an isolated PostgreSQL service.
- [x] P02.07 Provide an example configuration without actual secrets.
- [x] P02.08 Implement `GET /health/live` in Axum.
- [x] P02.09 Add a Rust unit test run by CI.
- [x] P02.10 Configure format, Clippy, tests, and dependency auditing according to policy.
- [x] P02.11 Protect branches, require PRs, and enable dependency review as appropriate.
- [x] P02.12 Record stack choices and tradeoffs in ADR-001.

**Exit:** `cargo test`, local PostgreSQL startup, and real PR checks work. Verify existing partial setup before claiming completion.

**Verified complete:** 2026-10-22 on `7e0b847`; Rust tests, isolated PostgreSQL
startup/lifecycle, and all four real PR checks passed. See the
[exit verification record](completed/2026-10-22-stack-adr.md#phase-02-exit-verification-2026-10-22).

### Phase 03 — Financial model and migrations

- [x] P03.01 Implement Rust `Money` and `Currency` types without floats.
- [x] P03.02 Test addition, subtraction, bounds, scale, and rounding.
- [x] P03.03 Define effective dates, timestamps, and time zones. See the [date specification](../product/dates-and-time-zones.md); no runtime implementation is implied.
- [x] P03.04 Create migrations for `users` and minimal session/identity storage. See [identity storage](../architecture/identity-storage.md) and the [verified execution record](completed/2026-10-24-user-identity-storage.md).
- [x] P03.05 Create `accounts` migration and ownership constraints. See [account storage](../architecture/account-storage.md) and the [verified execution record](completed/2026-10-24-accounts-storage.md).
- [x] P03.06 Create `categories` migration with built-in categories. See [category storage](../architecture/category-storage.md) and the [verified execution record](completed/2026-10-24-categories-storage.md).
- [x] P03.07 Create `transactions` migration. See [transaction storage](../architecture/transaction-storage.md) and the [verified execution record](completed/2026-10-24-transactions-storage.md).
- [x] P03.08 Create `account_movements` migration, foreign keys, and indexes. See [account movement storage](../architecture/account-movement-storage.md) and the [verified execution record](completed/2026-10-25-account-movements-storage.md).
- [x] P03.09 Create `transaction_audit` migration. See [audit storage](../architecture/transaction-audit-storage.md) and the [verified execution record](completed/2026-10-25-transaction-audit-storage.md); automatic auditing remains pending.
- [ ] P03.10 Add checks for amounts, operation kinds, supported currencies, account-type income restrictions, and user/date indexes.
- [ ] P03.11 Test all migrations against a fresh PostgreSQL instance.
- [ ] P03.12 Test SQL constraints and cross-user relationship restrictions.

**Exit:** Reproducible schema, with key invariants enforced by code and database.

**P03.01–P03.02 verified locally:** 2026-10-24, with 20 domain acceptance test
groups and the existing API liveness test passing on Rust 1.99.1. See the
[TDD execution record](completed/2026-10-24-money-specification.md). The remaining
Phase 03 tasks and its schema exit criteria are not complete.

**P03.03 defined:** 2026-10-24, with confirmed calendar policies, explicit formats,
bounds and independently checked midnight/leap-date/DST examples. See the
[specification record](completed/2026-10-24-date-semantics.md). Date types, audit fields,
profile storage and database constraints are not implemented by this definition.

### Phase 04 — Authentication and isolation

- [x] P04.01 Document the identity strategy: first-party credentials or OIDC provider. See the [initial identity decision](../product/identity-and-access.md) and [ADR-004](../architecture/decisions/004-first-party-identity.md).
- [ ] P04.02 Implement user registration.
- [ ] P04.03 Implement secure login and Argon2id password hashing if managing passwords.
- [ ] P04.04 Implement session/refresh-token rotation and revocation.
- [ ] P04.05 Implement authenticated identity middleware.
- [ ] P04.06 Enforce ownership on all resource reads, writes, and relationships.
- [ ] P04.07 Define consistent typed API errors and request IDs.
- [ ] P04.08 Add rate limits and protections to sensitive endpoints.
- [ ] P04.09 Test expired/revoked tokens and refresh-token reuse.
- [ ] P04.10 Verify user A cannot read or change user B's resources.
- [ ] P04.11 Implement logout and global session revocation.
- [ ] P04.12 Design email verification and password recovery if passwords are owned.

**Exit:** Secure, isolated accounts and negative authorization tests.

**Identity direction:** First-party email/password credentials, open registration
and verified email before financial account creation or posting movements. This
defines authentication direction; P03.04 has a separate [storage implementation](../architecture/identity-storage.md). P04.02–P04.12 remain pending.
See the [P04.01 documentation record](completed/2026-10-24-first-party-identity.md).

### Phase 05 — Accounts and simple transactions (first vertical slice)

- [ ] P05.01 Model accounts and account types in the domain.
- [ ] P05.02 Implement account creation with name and currency.
- [ ] P05.03 Post a special opening-balance movement when nonzero.
- [ ] P05.04 Implement account listing scoped to the user.
- [ ] P05.05 Query account balances from posted account movements.
- [ ] P05.06 Implement account renaming and archiving.
- [ ] P05.07 Implement income and expense categories.
- [ ] P05.08 Record income without an account.
- [ ] P05.09 Record ordinary income linked to a normal account; reject savings destinations on creation, edits, and later assignment.
- [ ] P05.10 Record an expense without an account.
- [ ] P05.11 Record an expense linked to an account.
- [ ] P05.12 Implement paginated transactions and date/category/account filters.
- [ ] P05.13 Implement atomic editing and voiding with movement corrections.
- [ ] P05.14 Add tests for balances, accountless entries, and rejection of ordinary income into savings on creation, edits, and reassignment.

**Exit:** Create user → create account → record income/expense → query correct balance via API.

### Phase 06 — Safe transfers

- [ ] P06.01 Define the transfer contract: sent/received amounts and currencies, optional directional rate, precision/discrepancy rules, and rejection without partial effects.
- [ ] P06.02 Test that a transfer creates a source debit and destination credit with the correct amounts and currencies.
- [ ] P06.03 Implement a single atomic SQL transaction for transfers.
- [ ] P06.04 Block transfers involving another user's account.
- [ ] P06.05 Block same-source-and-destination transfers.
- [ ] P06.06 Support manual SEK/EUR/USD cross-currency transfers with required sent/received amounts and an optional rate; reject unsupported currencies.
- [ ] P06.07 Implement `Idempotency-Key` for transfer POST requests.
- [ ] P06.08 Force a failure between writes and verify full rollback.
- [ ] P06.09 Test retries, concurrency, and duplicate submissions.
- [ ] P06.10 Exclude transfers from income and expense statistics.

**Exit:** Transfers are atomic, secure, idempotent, and correctly reported.

### Phase 07 — Recurring expenses and due dates

- [ ] P07.01 Define the policy for days 29/30/31 and February.
- [ ] P07.02 Define the recurrence object, states, and lifecycle.
- [ ] P07.03 Migrate `recurring_rules` and `recurring_occurrences`.
- [ ] P07.04 Implement a pure next-occurrence calculator.
- [ ] P07.05 Test weekly, monthly, quarterly, and annual recurrence.
- [ ] P07.06 Test leap years and year transitions.
- [ ] P07.07 Implement creating, editing, and pausing recurrence rules.
- [ ] P07.08 Implement idempotent due-occurrence generation.
- [ ] P07.09 Add a safe daily worker/job with concurrency control.
- [ ] P07.10 Convert a confirmed occurrence into a posted transaction.
- [ ] P07.11 Support skipping or postponing an occurrence.
- [ ] P07.12 Prove future planned occurrences never change current balances.

**Exit:** Upcoming payment calendar with no premature accounting.

### Phase 08 — Interest-bearing savings

- [ ] P08.01 Define optional interest terms, explicitly identified nominal annual rates, effective-dated changes, daily/monthly/quarterly/annual payouts, day count, balance timing, capitalization, and rounding before calculation.
- [ ] P08.02 Migrate `savings_terms` and `savings_rate_periods`.
- [ ] P08.03 Allow savings-account interest terms to be configured.
- [ ] P08.04 Implement a pure daily exact-decimal interest calculator.
- [ ] P08.05 Implement forecasts for daily and monthly payouts with explicit annual-rate, day-count, capitalization, and rounding rules.
- [ ] P08.06 Support quarterly/annual payouts and configurable capitalization.
- [ ] P08.07 Test uneven month lengths, leap years, and mid-period deposits/withdrawals.
- [ ] P08.08 Specify whether negative rates are permitted; test zero rates, historical/future and mid-period rate changes, and the resulting acceptance/rejection policy.
- [ ] P08.09 Add an explainable forecast endpoint with breakdown.
- [ ] P08.10 Record actual interest exactly once in the receiving account, preserving the distinction from forecast interest and savings transfers.
- [ ] P08.11 Ensure forecast operations never affect posted balances.
- [ ] P08.12 Compare calculated outputs to hand-worked examples and document limitations.

**Exit:** Transparent projections and correctly posted real payouts.

### Phase 09 — Insights and reporting

- [ ] P09.01 Define dashboard metrics, selected-date pending-payment boundaries, and per-account planning difference; distinguish posted balance, net flow, savings, and estimates.
- [ ] P09.02 Query aggregate income and expenses by period.
- [ ] P09.03 Query expense distribution by category.
- [ ] P09.04 Query monthly net flow and history.
- [ ] P09.05 Query account balance history and implement per-account pending-payment totals and planning differences through a selected date, keeping unassigned occurrences visible.
- [ ] P09.06 Separate currencies in reports without implicit FX.
- [ ] P09.07 Exclude transfers, account openings, and voided entries from normal income/spending.
- [ ] P09.08 Include accountless spending in financial reports.
- [ ] P09.09 Test empty periods and pagination/filter combinations.
- [ ] P09.10 Profile and optimize queries using indexes and `EXPLAIN`.

**Exit:** Dashboard amounts reconcile with business transactions and account postings.

### Phase 10 — Shared mobile foundation (KMP)

- [ ] P10.01 Create KMP shared module and Android/iOS targets.
- [ ] P10.02 Create minimal Android and iOS hosts that consume shared logic.
- [ ] P10.03 Set up `commonTest` and run a first test on both targets.
- [ ] P10.04 Model `Money`, `Account`, `Transaction`, and `SavingsForecast` in Kotlin.
- [ ] P10.05 Serialize financial amounts as exact decimal strings.
- [ ] P10.06 Add Ktor HTTP client and typed API errors.
- [ ] P10.07 Define `SessionStore` with native Keychain/Keystore implementations.
- [ ] P10.08 Provide account/transaction repositories with test fakes.
- [ ] P10.09 Implement shared form validation.
- [ ] P10.10 Share presentation state logic where genuinely useful.
- [ ] P10.11 Test shared JSON contract fixtures in Rust and KMP.
- [ ] P10.12 Run shared tests for Android and iOS targets in CI.

**Exit:** Both apps compile with shared logic and unit tests.

### Phase 11 — Android native UI

- [ ] P11.01 Build Compose navigation, theme, and base components.
- [ ] P11.02 Implement login/registration through shared repositories.
- [ ] P11.03 Implement onboarding to the first account.
- [ ] P11.04 Implement account list/details.
- [ ] P11.05 Implement dated income/expense forms with optional account.
- [ ] P11.06 Implement transactions list and filters.
- [ ] P11.07 Implement transfer form and confirmation.
- [ ] P11.08 Implement recurrence and payment confirmation.
- [ ] P11.09 Implement savings terms, forecasts, and actual interest receipts.
- [ ] P11.10 Implement dashboard/statistics with per-account pending payments and labeled funding differences, separate from posted balances and grouped by currency.
- [ ] P11.11 Verify accessibility, offline/error states, and recovery flows.
- [ ] P11.12 Add Compose and Keystore-specific tests.

**Exit:** End-to-end MVP use cases work in the native Android application.

### Phase 12 — iOS native UI

- [ ] P12.01 Create the Xcode SwiftUI project linked to KMP shared logic.
- [ ] P12.02 Build SwiftUI navigation, theme, and base components.
- [ ] P12.03 Implement login/registration using Keychain-backed sessions.
- [ ] P12.04 Implement first-account onboarding.
- [ ] P12.05 Implement account list/details.
- [ ] P12.06 Implement dated income/expense forms with optional account.
- [ ] P12.07 Implement transactions list and filters.
- [ ] P12.08 Implement transfer form and confirmation.
- [ ] P12.09 Implement recurrence and due-payment workflows.
- [ ] P12.10 Implement savings terms, forecasts, and posted interest receipts.
- [ ] P12.11 Implement dashboard/statistics with per-account pending payments and labeled funding differences, separate from posted balances and grouped by currency.
- [ ] P12.12 Add XCTest/SwiftUI integration and Keychain tests.

**Exit:** Equivalent core use cases with genuinely native iOS UX.

### Phase 13 — Shared E2E, regression, and resilience

- [ ] P13.01 Standardize accessible test identifiers across UIs.
- [ ] P13.02 Install Maestro and configure Android/iOS test environments.
- [ ] P13.03 Add shared registration/login flow.
- [ ] P13.04 Add shared create-account flow.
- [ ] P13.05 Add shared accountless-expense flow.
- [ ] P13.06 Add shared linked income/expense flow.
- [ ] P13.07 Add shared transfer flow validating both resulting balances.
- [ ] P13.08 Add shared recurrence-confirmation flow.
- [ ] P13.09 Add shared actual-interest-payment flow.
- [ ] P13.10 Provide isolated test-data seed and reset mechanisms.
- [ ] P13.11 Add platform-specific subflows for differing permissions/keyboards.
- [ ] P13.12 Run the same core flows on Android emulator and iOS simulator.
- [ ] P13.13 Add security tests for unauthorized access, injection, enumeration, rate limits, and token replay/theft scenarios.
- [ ] P13.14 Add basic load tests and PostgreSQL recovery tests.

**Exit:** Repeatable regression suite for key financial scenarios on both platforms.

### Phase 14 — Operations, privacy, and release

- [ ] P14.01 Define separate local, staging, and production environments/secrets.
- [ ] P14.02 Build a non-root API container with health checks.
- [ ] P14.03 Deploy PostgreSQL with least privilege and private networking.
- [ ] P14.04 Set up HTTPS, security headers, and reverse-proxy configuration.
- [ ] P14.05 Apply database migrations through a controlled procedure.
- [ ] P14.06 Configure encrypted off-server backups with retention.
- [ ] P14.07 Test a complete backup restore into staging.
- [ ] P14.08 Add privacy-safe metrics, traces, and alerts.
- [ ] P14.09 Implement privacy notice, user data export, and deletion according to applicable requirements.
- [ ] P14.10 Perform OWASP MASVS/ASVS and dependency checks.
- [ ] P14.11 Capture errors without leaking financial or identity data.
- [ ] P14.12 Prepare Android/iOS signing, certificates, and distribution.
- [ ] P14.13 Conduct a limited beta and fix priority issues.
- [ ] P14.14 Publish MVP with documented rollback and basic observability.

**Exit:** Deployed, recoverable, tested software ready for controlled use.

### Phase 15 — Improvements after MVP

- [ ] P15.01 Add category/month budgets.
- [ ] P15.02 Add savings goals and progress tracking.
- [ ] P15.03 Add local upcoming-payment reminders.
- [ ] P15.04 Export transactions to CSV/JSON.
- [ ] P15.05 Add advanced search and filters.
- [ ] P15.06 **Superseded planning slot:** effective-dated and mid-period rate changes belong to MVP P08.01/P08.08. Retain this ID for traceability; it is not a separate post-MVP feature or implementation blocker.
- [ ] P15.07 Add English/Spanish/Swedish language preferences.
- [ ] P15.08 Gather feedback and reprioritize roadmap issues.

**Exit:** Improved release guided by actual user feedback.

## 8. Acceptance examples required before public release

| ID | Scenario | Required result |
| --- | --- | --- |
| FIN-001 | New account opened with 1,000 SEK | Balance = 1,000 SEK; normal monthly income = 0 |
| FIN-002 | 120 SEK expense with no account | Monthly spending increases by 120 SEK; no account balance changes |
| FIN-003 | Link FIN-002 expense to account A | Account A changes by −120 SEK **once**; monthly spending stays +120 SEK |
| FIN-004 | 2,000 SEK income linked to account A | Account A +2,000 SEK; reported monthly income +2,000 SEK |
| FIN-005 | Transfer 300 SEK from A to B | A −300, B +300; ordinary income/spending unchanged |
| FIN-006 | Retry FIN-005 with the same idempotency key | No additional postings |
| FIN-007 | Force a failure between transfer postings | Neither balance changes |
| FIN-008 | Plan next month's rent payment | Before confirmation: zero new real spending; balances unchanged |
| FIN-009 | Confirm the same rent occurrence twice | Exactly one real transaction |
| FIN-010 | Interest projected but not credited | Forecast shown; account balance unchanged |
| FIN-011 | Record confirmed credited interest | Balance increases by the exact posted amount; interest income recorded |
| FIN-012 | User A requests account owned by user B | Access denied without disclosing B's data |
| FIN-013 | One account SEK and another EUR with no FX rates | Totals remain separated by currency |
| FIN-014 | Monthly recurrence begins on January 31 | February due date follows documented month-end policy |
| FIN-015 | Void a previously posted income | Account balance and financial reports reconcile correctly |
| FIN-016 | Reinstall app and sign in | Financial records are recovered from the authenticated PostgreSQL-backed API |
| FIN-017 | Send 200 SEK from a 1,000 SEK account and receive 18 EUR into a 100 EUR account, with no rate entered | Balances become 800 SEK and 118 EUR; one transfer, no income/spending; retries do not duplicate either posting |
| FIN-018 | Send 10 EUR using a helper rate of 1.10 USD per EUR, then confirm 10.95 USD received | Debit 10 EUR and credit 10.95 USD; optional rate does not overwrite actual amounts or infer a fee |
| FIN-019 | Post or reassign 200 SEK of ordinary work income into savings | Reject without changing entries or balances; an existing accountless income stays accountless and remains one report item |
| FIN-020 | Payment account has 300 SEK, another has 2,000 SEK, and 800 SEK is pending from the first account | First account's planning difference is −500 SEK; its posted balance remains 300 SEK despite 2,300 SEK combined funds |
| FIN-021 | Rate changes from 2% to 3% mid-period on an account with daily payouts | Retain effective-dated rate history and apply specified period rules to forecasts; daily payment does not imply a daily quoted rate or automatically post interest |
| FIN-022 | Savings holds 1,000 SEK, forecast interest is 5 SEK, and 4 SEK is actually credited and confirmed twice | Savings becomes 1,004 SEK and interest income is 4 SEK exactly once; forecast and its difference create no posting |

## 9. Proposed repository layout

This is a **target layout**, not a claim that the directories or artifacts currently exist:

```text
cuenora/
├── AGENTS.md
├── .github/
│   └── workflows/
├── .devcontainer/
├── backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── rust-toolchain.toml
│   ├── crates/
│   │   ├── domain/          # Pure financial rules and types
│   │   ├── application/     # Use cases and domain ports, if needed
│   │   ├── infrastructure/  # SQLx and PostgreSQL adapters, if needed
│   │   └── api/             # Axum executable and HTTP adapters
│   ├── migrations/
│   └── tests/
├── mobile/
│   ├── shared/              # Kotlin Multiplatform
│   │   └── src/
│   │       ├── commonMain/
│   │       ├── commonTest/
│   │       ├── androidMain/
│   │       └── iosMain/
│   ├── androidApp/          # Jetpack Compose
│   └── iosApp/              # SwiftUI / Xcode
├── contracts/
│   └── openapi.yaml
├── tests/
│   ├── fixtures/            # Common JSON contract data
│   └── e2e/                 # Maestro flows and platform extensions
├── infra/
│   └── docker/
├── docs/
│   ├── plans/
│   │   ├── README.md
│   │   ├── master-plan.md
│   │   ├── TEMPLATE.md
│   │   ├── active/
│   │   └── completed/
│   ├── product/
│   ├── architecture/
│   └── development/
├── README.md
├── CONTRIBUTING.md
├── SECURITY.md
└── LICENSE
```

The layout must not induce circular dependencies: `domain` is transport/persistence agnostic, application use cases depend on abstractions, infrastructure supplies adapters, and API composes the application. Start with fewer crates if that reduces unnecessary complexity.

## 10. Definition of Done (each task / PR)

A task is not complete until the applicable checks below are satisfied:



- [ ] Requested behavior and relevant edge cases are implemented.
- [ ] Unit tests exist; database work includes integration tests using isolated real PostgreSQL.
- [ ] API changes update OpenAPI and relevant shared fixtures.
- [ ] Critical mobile flows update Maestro or appropriate platform tests, or document why not.
- [ ] Formatting, linting, building, and relevant automated tests pass.
- [ ] Authorization, input validation, and errors are reviewed where applicable.
- [ ] Schema changes include reproducible migrations and a safe deployment/rollback plan.
- [ ] Documentation or ADRs reflect any actual decision changes.
- [ ] No secrets, personal financial records, or sensitive data appear in source, logs, or artifacts.
- [ ] Behavior is reproducible in an isolated environment; staging where justified.
- [ ] Verification commands and actual outcomes are recorded in the PR or focused execution plan.

## 11. Key risks and early mitigations

| Risk | Mitigation |
| --- | --- |
| Divergent Android/iOS logic | Kotlin Multiplatform shared logic and `commonTest`; separate native UI only |
| Financial rules duplicated between clients and server | Server remains authoritative; shared contract fixtures for consistency |
| Partial transfer corrupts balances | Single SQL transaction, invariants, failure injection, rollback tests |
| Double charging by recurrence or retried requests | Explicit confirmation, unique constraints, idempotent writes |
| Forecast interest counted as earned interest | Separate states, explicit posted interest payment, documented terms |
| Cross-user data exposure | Owner-scoped reads/writes and references; adversarial tests |
| Incorrect currency arithmetic or rounding | Exact decimals, currency scales, round-off policies, edge tests |
| Premature offline sync or bank connectivity scope | Exclude from MVP until requirements are validated |
| iOS CI resources unavailable or expensive | Arrange macOS/Xcode support when KMP/iOS work starts |
| Disaster recovery fails | Encrypted offsite backups plus real restore drills |
| Privacy obligations overlooked | Data minimization, transparent retention, access controls, export, and erasure processes |

## 12. Future feature candidates

**Likely next steps:** Category budgets, savings goals, reminders, portable exports, CSV import, customizable categories, labels, shared expenses, manual reconciliation.

**Larger investments:** Offline-first conflict resolution, region-appropriate open banking, dated FX conversion, receipt OCR, detected subscriptions, widgets, push notifications, automated categorization, advanced cash-flow projections.

**Intentionally avoided for now:** Actual payments or custody, automated regulated financial advice, investment return guarantees, AI systems with ungoverned access to personal financial records.

## 13. Current implementation order

Phase 00 product documents and the verified Phase 02 scaffold are complete;
financial behavior and mobile clients remain unimplemented. Start from
[PRD v0.1](../product/prd.md) and the [value-ordered backlog](../product/backlog.md).
Do not repeat completed repository setup or confuse its unit test with a Money test.

1. Complete the remaining Phase 01 flow and wireframe work against the agreed
   account-funding problem; use it to guide screens without changing financial rules.
2. Money scale, bounds and rounding are implemented and tested in P03.01–P03.02.
   P03.03 date semantics are defined; implement their validation alongside future
   date types and application/persistence work.
3. Build isolated persistence and ownership constraints (remaining Phase 03),
   then authenticated access (Phase 04), before exposing financial writes.
4. Deliver accounts, openings, income/expenses, and corrections (Phase 05),
   enforcing savings restrictions from the first slice.
5. Deliver same-currency and manual cross-currency transfers (Phase 06), then
   recurring schedules and explicit payment confirmation (Phase 07).
6. Specify metric boundaries in P09.01 and implement P09.05 once account balances
   and recurring occurrences exist. This account-funding comparison does not
   depend on the interest calculator and need not wait for all of Phase 08.
7. Implement Phase 08 interest only after its calculation policies are explicit;
   complete the remaining reporting work and native mobile experiences using
   shared contracts (Phases 09–12).
8. Keep unit/integration/platform checks with each behavior. Phase 13 broadens
   end-to-end coverage; Phase 14 release, security, backup, and restore evidence
   is required before controlled use. Phase 15 remains post-MVP.

These are dependency boundaries, not deadlines or instructions to implement an
entire phase in one PR. Choose one focused roadmap task with its tests and docs.
P15.06 is retained solely as a superseded alias for MVP interest work.

## 14. Technical references

- Kotlin Multiplatform native UI/shared logic: https://kotlinlang.org/docs/tutorials/native/mpp-ios-android.html
- Kotlin Multiplatform tests: https://kotlinlang.org/docs/multiplatform/multiplatform-run-tests.html
- Maestro: https://maestro.dev/
- Axum: https://docs.rs/axum/latest/axum/
- SQLx: https://docs.rs/sqlx/latest/sqlx/
- PostgreSQL numeric types: https://www.postgresql.org/docs/current/datatype-numeric.html
- OWASP MASVS: https://mas.owasp.org/MASVS/

## Next planning artifact

Use the completed [PRD v0.1](../product/prd.md), [backlog](../product/backlog.md), and [worked examples](../product/worked-examples.md). The [money execution plan](completed/2026-10-24-money-specification.md) records P03.01–P03.02 semantics and observed RED/GREEN tests; the [date plan](completed/2026-10-24-date-semantics.md) records the P03.03 definition. The [identity strategy](../product/identity-and-access.md) completes P04.01's direction. P03.04 minimal identity/session migrations are implemented and verified locally; see the [execution record](completed/2026-10-24-user-identity-storage.md). P03.05 account storage is implemented and verified locally; see its [execution record](completed/2026-10-24-accounts-storage.md). P03.06 global built-in categories are implemented and merged; see [category storage](../architecture/category-storage.md) and its [execution record](completed/2026-10-24-categories-storage.md). P03.07 transaction event storage is implemented and merged; see [transaction storage](../architecture/transaction-storage.md) and its [execution record](completed/2026-10-24-transactions-storage.md). Next, scope P03.08 account movements; account application actions and authentication remain later roadmap work. Record concrete technical decisions in ADRs when made, without treating unsettled interest, recurrence-calendar, or concurrency policies as approved. Keep Phase 01 flows aligned with per-account funding needs. Create focused implementation plans from [TEMPLATE.md](TEMPLATE.md) as required.
