# Cuenora — Master Project Plan

> **Status:** Living roadmap; approved scope and verified implementation are distinguished below.
> **Version:** 0.10 — updated 2026-11-06 with operator-reported recurrence generation and first automatic staging execution; latest integrated follow-up `785025c` (PR #115), deployed image digest not recorded.
> **Origin:** English adaptation of the original 2026-10-21 personal-finance master plan; original task IDs (`P00.01`–`P15.08`) are preserved.
> **Project:** [cuenora/cuenora](https://github.com/cuenora/cuenora).
> **Purpose:** Build an open-source, non-commercial personal-finance app for Android and iOS with a Rust API, PostgreSQL, native user interfaces, and shared mobile logic and test cases.

**Reading note:** A checked task means its stated deliverable was verified, not that its whole phase or the product is usable. Design, storage, domain, API and mobile delivery are separate milestones. Approved decisions and implemented schemas below are not open proposals; explicitly future contracts and unresolved policies still require decisions before implementation. Local verification does not imply merge or production readiness.

The [PRD v0.1](../product/prd.md), [backlog](../product/backlog.md), and linked
Phase 00 decisions, later product specifications and accepted ADRs define the current product baseline. Checked product tasks document
decisions, not implemented financial behavior. The 186 original task IDs are
retained; P15.06 is a superseded slot, not an additional implementation task.

### Current delivery snapshot

| Area | Verified implementation and integration evidence | Remaining boundary |
| --- | --- | --- |
| Product, UX and scaffold (Phases 00–02) | Accepted product/design decisions, static wireframes, Rust workspace and repository tooling | Native clients; wireframes are not working screens |
| Financial foundations and accounts (Phases 03/05) | Exact money, append-only movements/audit, confirmed profile zones, owner-scoped accounts, dated openings, balances, categories, income/expenses, history, edits and voids; merged through PR #85 | Production migrations and operational validation |
| Authentication and isolation (Phase 04) | Registration, login, rotation/revocation, ownership checks, throttling, session-bound email verification, recovery and browser sign-in/confirmation; PR #104–#107 | Remaining recovery/browser journeys and full proxy validation in staging; production mail configuration; no browser dashboard or durable browser session |
| Transfers and recurrence (Phases 06–07) | Atomic permanent-retry transfers, optional exact quotes, calendar rules, generation job, confirmation, skip/postpone and no-premature-posting regressions; PR #85/#87–#95; packaged job/helper through #113/#115 and operator-reported manual/automatic staging execution | Continued scheduling/monitoring and deployment traceability; planned items never post automatically |
| Savings (Phase 08) | Immutable terms/rates, exact daily/payout calculators, explainable read-only forecast, actual interest and isolation regressions; PR #96–#101 | Documented forecast limits remain; no bank reconciliation or automatic interest posting |
| Reports (Phase 09) | Currency-separated flows, category distribution, monthly series, balance history, account planning and query/regression evidence; PR #102–#103 | Native presentation and production workload validation |
| Runtime and operations (Phase 14) | Alpha.5 published for AMD64/ARM64 through PR #111; operator-reported healthy container, public liveness/readiness and controlled PostgreSQL stop/start recovery; initial schema, restricted API-role cutover and scoped identity/account/transfer/recurrence checks | Complete least-privilege/network evidence, full proxy/traffic-admission configuration, production/upgrade migrations, backups/restore, monitoring, privacy/security and rollback evidence |
| Mobile and broad E2E (Phases 10–13) | Shared foundations/tests and minimal native hosts; P11.01 Android theme/navigation/components plus P11.02 authentication/session lifecycle locally verified with isolated shared/native tests; native light/dark foundation launch verified | Functional financial screens and platform/shared E2E remain pending; iOS host stays minimal; no broad accessibility or supported-device certification |

The alpha.5 source was `05bdd7a` (PR #111), following PR #110 staging
helpers/runbooks, PR #109 publication support and PR #108 roadmap reconciliation.
Subsequent merged follow-ups package the recurrence executable (#113), document
the runtime-role cutover (#114), and add the staging helper (#115, `785025c`).
The operator reported publishing and deploying a job-capable image; its exact
tag/digest has not been recorded or independently checked here.
The completed [alpha publication record](completed/2026-11-04-docker-alpha.md) and
[health/readiness record](completed/2026-11-04-health-readiness.md) preserve merge,
verification and subsequent deployment evidence. Authenticated staging smokes and
container status below are operator-reported; public endpoint and registry checks
were independently observed. These are not a complete infrastructure audit.

### Staging evidence — 2026-11-04

| Roadmap coverage | Observed evidence | Completion boundary |
| --- | --- | --- |
| P14.05 — initial staging bootstrap | Operator initialized the empty staging database using all 23 SQL migrations pinned to the alpha revision, with empty-schema guard, error-stop and one transaction. Subsequent anonymous authentication lookup changed from 503 to the expected invalid-token 401. See the [bootstrap runbook](../development/INITIALIZE_ALPHA_DATABASE.md). | Initial staging bootstrap only. No automatic startup migration, upgrade/history runner, production migration or restore drill is claimed. |
| P04.02/P04.03/P04.04/P04.12 — existing checks reinforced | Operator completed registration/login, received verification mail, refreshed the session, confirmed email with 204 and rejected challenge replay with 400 for two distinct test users. | Terminal identity smoke; not full password recovery, browser/mobile E2E or stolen-token testing. |
| P04.06/P04.10 — existing checks reinforced | Corrected [account-isolation helper](../development/STAGING_ACCOUNT_ISOLATION.md) passed owner controls, bidirectional foreign reads/renames/cursors, unchanged public account snapshots and current-session logout. | Account metadata only; not every financial resource or database-state hash. Earlier failed-run sessions are not confirmed revoked. |
| P05.03/P06.03/P06.07/P06.09 — existing checks reinforced | [Bounded transfer helper](../development/STAGING_TRANSFER_CONCURRENCY.md) passed four equal-key submissions producing one event, four distinct-key operations, exact 86.00/14.00 SEK balances, both account histories and rejected-write snapshot checks. | Four requests maximum in parallel, synthetic data and successful final public invariants. No staging DB failure injection or sustained load proof. |
| P14.02 — deployed checks verified | Alpha.5 contains the non-root local liveness probe and Docker HEALTHCHECK; the operator reported the deployed container healthy. Public checks observed live/ready 200, ready 503 with live 200 during PostgreSQL stop, then both 200 after start. See the [completed record](completed/2026-11-04-health-readiness.md). | No continuous-uptime/restart-counter proof, exact Swarm configuration audit, readiness-only traffic gate or backup restoration is claimed. |
| P14.04 — partial | Public HTTP redirects to HTTPS; validated TLS 1.3, HSTS and restrictive identity-page security headers. | Origin ingress isolation, immediate proxy trust and spoofed-header/rate-budget behavior remain unverified. |
| P13.13/P13.14 — partial or pending | Scoped ownership, challenge replay and bounded concurrency checks passed. | Injection, enumeration, complete token/rate-limit coverage, basic load and PostgreSQL restore/crash-recovery are not established by these smokes. The observed stop/start readiness recovery is narrower; mobile shared flows also remain pending. |

Historical alpha.5 publication (2026-11-04): [successful workflow run](https://github.com/cuenora/cuenora/actions/runs/41900550725).
Manifest digest: `sha256:b823ddb5f0921910018180b40cbff7a70895868b41815c78be11a6a7f1480b13`.
The staging database contains synthetic test accounts, opening movements and
transfers; these helpers deliberately do not delete their records. No personal
financial data or production-readiness claim is introduced by this update.

### Staging runtime-role follow-up — 2026-11-05

The operator reported cutting the API over from the administrator to
`cuenora_api`, with explicit table/operation grants and no schema ownership.
Private-hostname password authentication and sampled effective privileges were
verified by operator-supplied SQL output; TEMPORARY was revoked from PUBLIC in
the dedicated database. Both public health endpoints reportedly returned 200/OK.
The bounded transfer helper passed after cutover, including exact 86.00/14.00 SEK
balances, permanent-key replay and rejected-write snapshots. Synthetic records
remain. The operator also supplied a successful password-recovery smoke: reset
204, replay/old-password/old-refresh rejection and new-password login, followed by
confirmation that the password-change notification arrived. See the
[runbook](../development/POSTGRES_RUNTIME_ROLE.md) for reproducible
steps, evidence provenance and remaining boundaries.

This does not prove every restricted-role workflow, separately scheduled job
credentials, independent network isolation, full proxy security or production
readiness. P14.03 remains unchecked; no other roadmap checkbox changes.

### Staging recurrence follow-up — 2026-11-05–06

After reporting publication/deployment of an image containing the job, the
operator supplied successful deployed `--help` output and a passing interactive
[staging recurrence check](../development/STAGING_RECURRENCE_GENERATION.md).
One bounded synthetic rule produced aggregate pending `10.00` SEK after both
manual runs, with zero synthetic balance and unchanged public financial snapshots.
This is not direct occurrence-count/identity or database movement/audit evidence.

The operator then configured `cuenora-recurrence-daily` in the API application's
Dokploy scheduler: `15 2 * * *`, explicit UTC, Sh, enabled, executing
`/usr/local/bin/cuenora-recurrence-job` with the API container's restricted-role
environment. Manual task execution and the first reported automatic run on
2026-11-06 at 02:15 UTC both logged
`status=completed rules_processed=1 occurrences_created=0 rules_failed=0`
and command success. The automatic trigger/time is operator-reported; the pasted
log itself has no timestamp. See the [schedule evidence](../development/DOKPLOY_RECURRENCE_JOB.md).

The single eligible slot was already generated; these later zero-creation runs
do not prove generation of a newly due slot on a later date. No payment was
confirmed; synthetic records remain. Exact image digest, continued job monitoring,
proxy/network review, backups/restore and production validation remain pending.
No roadmap checkbox changes are introduced by this evidence update.

### Staging API ingress and proxy follow-up — 2026-11-06

The operator supplied private-network membership and TCP evidence after moving
the existing API and PostgreSQL services off Dokploy's shared network, retaining
their running image digests and the original PostgreSQL volume. Sampled public
readiness stayed healthy; shared-network TCP probes could not connect to the two
tested private endpoints while intended-path controls succeeded before and after.
Exact-peer trust was enabled only after the API-observed peer matched Traefik;
post-deploy runtime configuration and the same peer were verified.

Protected malformed-login checks returned 400 through the public chain and 503
for missing provenance or an untrusted peer with forged headers. A corrected,
bounded two-client test observed 20 admitted requests then 429 for client A,
continued 429 under changed XFF/real-IP headers, independent 400 for client B and
recovery for A after the quiet window. Final operator YAML readback retained the
Cloudflare allowlist on both routers and the task-DNS upstream. A final public
smoke observed HTTP-to-HTTPS 301, TLS 1.3 with successful certificate validation,
readiness and identity-page 200 responses, HSTS and restrictive identity headers.
Final direct-origin IPv4 checks returned 403 for HTTP and HTTPS, both normally
and with forged client-IP headers; origin HTTPS certificate validation succeeded.
See the
[evidence and maintenance guide](../development/STAGING_API_PROXY.md) for the
initial Cloudflare error-1000 test, provenance and limits of these observations.

This supersedes the unverified ingress/trust/rate-budget boundary in the older
2026-11-04 table for the tested staging scope only. Traefik's custom ingress
attachment still needs manual restoration after recreation; no recreation drill,
external IPv6 proof, global firewall hardening or production certification was
performed. P14.04 is completed for this verified staging scope, with manual proxy
restoration accepted and documented. P14.03 has its separate completion follow-up
below; production validation and the rest of Phase 14 are not completed by this record.

### Staging PostgreSQL runtime-role completion — 2026-11-06

The operator supplied read-only catalog output confirming restricted role
attributes, no other role memberships, no database/schema creation or temporary
privileges, and the expected effective operation grants on all 25 public tables.
The runtime owned none of those tables or the 15 public routines, could not
delegate their permissions and had no extra column-only grants. All public
routines were SECURITY INVOKER; no public sequences or custom default-grant
entries were present. The actual running API's private data-network IP matched
a PostgreSQL connection using `cuenora_api`.

The operator explicitly confirmed repeating the earlier scripts after runtime-role
cutover with the same successful outcomes for email verification, recurrence
confirmation/actions and savings. This is operator-confirmed workflow evidence,
not newly inspected full script output or an independent live reproduction.
Combined with the network checks above and earlier transfer/recovery/generation
evidence, this completes P14.03 for staging. See the
[runtime-role completion record](../development/POSTGRES_RUNTIME_ROLE.md#staging-completion-follow-up--2026-11-06).
This supersedes the pending staging boundaries in the 2026-11-05 role follow-up;
it does not certify production or complete Phase 14.

## 0. Vision and boundaries

### Product vision

Help individuals record, understand, and organize accounts, income, expenses, transfers, recurring payments, interest-bearing savings, goals, and financial trends.

The first user's concrete problem is fragmented money across multiple banks:
an account used for a scheduled payment can be short even when money exists in
another account. The MVP records activity manually and compares **each account's**
posted balance with its pending payments through a selected date. It does not
read live bank balances, pool currencies, execute payments or guarantee funds
will be available. Goals and proactive reminders remain post-MVP. See the
[target user](../product/target-user.md) and [prioritized stories](../product/user-stories.md).

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

- One user owns multiple accounts with exactly two MVP domain types: `Normal` and `Savings`. Cash/checking are uses of normal accounts, not additional types; credit-specific behavior is not part of the approved MVP. See the [account model](../product/accounts.md).
- Account names trim outer Unicode whitespace, preserve internal text/case/Unicode, reject empty results and remaining ASCII controls, and allow up to 255 Unicode scalar values. Duplicate names are allowed.
- Categories are a global, read-only built-in income/expense catalog, not user-editable records. Custom categories remain post-MVP; see [category storage](../architecture/category-storage.md).
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

## 2. Technology and architecture direction

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
- See [date semantics](../product/dates-and-time-zones.md) and [ADR-003](../architecture/decisions/003-date-semantics.md): profile zones are confirmed and stable; existing rules retain independent zones; realized dates cannot be future-dated at creation or explicit date edit. Runtime validation is implemented in dated financial operations; the original definition alone was not runtime proof.
- Calculations may retain greater precision than final postings; document rounding at settlement.

### Accounts and accountless entries

- Accounts have owner, type, name, currency, and active/archived status.
- Ordinary income may be assigned to normal accounts, not savings accounts; enforce this on creation, edits, and later linking of accountless entries.
- Accountless income or expense contributes to reports, never the aggregate of account balances.
- Linking, unlinking, or editing a posted transaction appends the required compensating/new account movements atomically and **exactly once**, without replacing historical postings.
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

- User financial records have an owner and auditable timestamps/changes; the global category catalog is shared read-only reference data, not user-owned financial data.
- Edit, void, or reverse posted entries consistently and transactionally; never leave orphaned or doubled account movements.
- Account movements are append-only; corrections add compensating/new postings rather than rewriting history. A transaction state change alone does not reverse its balance effect.
- The accepted [minimal audit](../architecture/decisions/007-minimal-transaction-audit.md) records the owner as actor, action, transaction, UTC instant and changed field names only. It does not retain values, descriptions or snapshots and cannot reconstruct previous transaction versions. Application writes must update events, postings and audit atomically.
- Archive accounts with history rather than silently deleting them. Data-erasure requests are a separate privacy workflow governed by applicable retention requirements.

## 4. Data model: implemented foundations and planned extensions

The table summarizes storage responsibilities. Recurrence, savings and operation-level
idempotency now have implemented migrations and application workflows. General display
preferences remain future work; the confirmed profile zone is stored on the identity.

| Table | Responsibility |
| --- | --- |
| `users` | Identity, email identity/comparison and verification state; confirmed profile-zone workflow implemented; general locale preferences remain future work |
| `password_credentials`, `user_sessions`, `session_refresh_tokens`, `session_access_tokens` | Password hashes, opaque digest-only tokens, rotation/reuse and session revocation; see [identity storage](../architecture/identity-storage.md) |
| `accounts` | User-owned account type, name, ISO currency, active/archived status |
| `categories` | Global immutable built-in income/expense classifications; no MVP category ownership or editing |
| `transactions` | Owned business operation, type, economic date, exact amount and currency (both sent and received for cross-currency transfers), optional directional transfer rate, category, description and state; atomic posting and operation-specific permanent retries implemented |
| `account_movements` | Signed, traceable account postings; **source of truth for account balances** |
| `transaction_audit` | Append-only owner-as-actor, action, transaction, UTC timestamp and changed field names; no value snapshots |
| `recurring_rules` | Schedule, amount, type, category, optional account, time zone, state |
| `recurring_occurrences` | Due date, planned/confirmed/skipped state, linked transaction, uniqueness |
| `savings_terms` | Account interest rules, day count, payout cadence, capitalization |
| `savings_rate_periods` | Nominal annual interest rate and validity interval |
| `account_creation_keys` | Implemented creation-only owner/key digest, original account result, lifetime retention (no TTL); other financial operations use their own implemented idempotency storage |
| `user_preferences` (planned) | Future display/locale/formatting preferences; confirmed profile zone already exists on `users` |

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

## 5. API contract: current surface and future scope

The [OpenAPI contract](../../contracts/openapi.yaml) is authoritative for the
implemented `/v1` health, authentication, profile, accounts, categories,
transactions/transfers, recurrence, savings and report routes. Browser identity
pages live under `/auth`; they do not provide a financial dashboard.

Implemented APIs include bounded pagination, ISO business dates, exact decimal-string
money and typed errors (`code`, `message`, `details`, `request_id`). Do not infer a
route from a product screen: data export/deletion and future mobile integrations
remain separate work. Preserve the versioning and retry boundaries in
[ADR-023](../architecture/decisions/023-versioned-account-creation.md) and each
operation's contract.

### Proposed MVP screens

- Short onboarding → register/sign in → verify email → create first account → home dashboard. Confirm the profile time zone before any operation needing a financial date (including a dated opening movement); metadata-only account creation does not itself require that zone.
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
- [x] P01.09 Explore an accessible initial color palette and typography. See the [initial exploration and high-fidelity references](../product/color-and-typography.md).
- [x] P01.10 Define UI design tokens, components, and interface states. See the [initial design system](../product/ui-design-system.md) and [merged execution record](completed/2026-10-25-ui-design-system.md).

**Exit:** User flows and low-fidelity screens that guide implementation.

### Phase 02 — Repository, tooling, and initial CI

- [x] P02.01 Create a Git monorepo and `README.md`.
- [x] P02.02 Select a license and contribution policy for open-source work.
- [x] P02.03 Create `backend`, `mobile`, `contracts`, `tests`, `infra`, and `docs` directories as needed.
- [x] P02.04 Configure a Rust 2024 Cargo workspace with `domain` and an `api` binary.
- [x] P02.05 Set up Rustfmt, Clippy, and task runner commands (`make` or `just`) if justified.
- [x] P02.06 Add local Docker Compose with an isolated PostgreSQL service.
- [x] P02.07 Provide an example configuration without actual secrets.
- [x] P02.08 Implement `GET /v1/health/live` in Axum.
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
- [x] P03.09 Create `transaction_audit` migration. See [audit storage](../architecture/transaction-audit-storage.md) and the [verified execution record](completed/2026-10-25-transaction-audit-storage.md); automatic auditing is implemented by subsequent financial posting workflows.
- [x] P03.10 Add checks for amounts, operation kinds, supported currencies, account-type income restrictions, and user/date indexes. See [financial constraints](../architecture/financial-constraints.md) and the [verified execution record](completed/2026-10-25-financial-constraints.md).
- [x] P03.11 Test all migrations against a fresh PostgreSQL instance. See the [fresh-instance verification](../development/SETUP.md#fresh-migration-verification-p0311): all eight migrations, two independent schema/seed comparisons, and 34 passing infrastructure tests on 2026-10-25.
- [x] P03.12 Test SQL constraints and cross-user relationship restrictions. See the [constraint verification](../development/SETUP.md#sql-constraints-and-owned-relationships-p0312): same-currency ownership controls, rejected inserts/edits, rollback checks and 36 passing infrastructure tests on 2026-10-25. Authenticated API isolation remains Phase 04 work.

**Exit:** Reproducible schema, with key invariants enforced by code and database.

**P03.01–P03.02 verified locally:** 2026-10-24, with 20 domain acceptance test
groups and the existing API liveness test passing on Rust 1.99.1. See the
[TDD execution record](completed/2026-10-24-money-specification.md). See P03.11–P03.12
above for subsequent schema and constraint verification.

**P03.03 defined:** 2026-10-24, with confirmed calendar policies, explicit formats,
bounds and independently checked midnight/leap-date/DST examples. See the
[specification record](completed/2026-10-24-date-semantics.md). Date types, audit fields,
profile storage and database constraints are not implemented by this definition.

### Phase 04 — Authentication and isolation

- [x] P04.01 Document the identity strategy: first-party credentials or OIDC provider. See the [initial identity decision](../product/identity-and-access.md) and [ADR-004](../architecture/decisions/004-first-party-identity.md).
- [x] P04.02 Implement user registration. See [registration setup](../development/SETUP.md#pending-user-registration-p0402) and [ADR-011](../architecture/decisions/011-bounded-registration-http.md): pending identity/credential persistence, duplicate safety, bounded off-executor hashing and disposable PostgreSQL HTTP verification. Login is implemented in P04.03 below; email verification is implemented; see the completed runtime records below.
- [x] P04.03 Implement secure login and Argon2id password hashing if managing passwords. See [login setup](../development/SETUP.md#opaque-login-p0403) and [ADR-015](../architecture/decisions/015-secure-login-http.md): exact-byte verification, shared bounded admission, atomic digest-only initial sessions and isolated PostgreSQL HTTP/concurrency proof.
- [x] P04.04 Implement session/refresh-token rotation and revocation. See [refresh setup](../development/SETUP.md#sliding-refresh-rotation-p0404), [ADR-016](../architecture/decisions/016-sliding-refresh-rotation.md) and [ADR-017](../architecture/decisions/017-refresh-http.md): one-time opaque rotation, sliding 30-day refresh validity, family-scoped replay revocation and isolated transaction/concurrency proof. Logout/global endpoints and bearer middleware remain separate tasks.
- [x] P04.05 Implement authenticated identity middleware. See [middleware integration](../development/SETUP.md#authenticated-identity-middleware-p0405), [ADR-018](../architecture/decisions/018-authenticated-identity.md) and the [completed execution record](completed/2026-10-27-authenticated-identity.md): read-only opaque access validation, typed server-derived user/session identity, expiry and family revocation checks; own/independent proof and native review passed, merged through [#57](https://github.com/cuenora/cuenora/pull/57). No protected production endpoint or financial authorization is added.
- [x] P04.06 Enforce ownership on all resource reads, writes, and relationships. Verified locally on 2026-11-02 for all currently routed financial resources: accounts, transactions/transfers, recurring rules/occurrences, savings and reports, plus authenticated profile-zone updates. Owner predicates use server-derived identity for resource access and related account selection; composite foreign keys additionally protect relationship integrity, not authenticated row access. All 20 disposable PostgreSQL HTTP harness targets passed in 309.988 seconds; no runtime or test changes were needed. Categories remain an authenticated global read-only catalog. Future endpoints must implement and verify the same boundaries alongside each operation; this does not implement absent endpoints or complete P04.10 automatically. See [ownership verification](../development/SETUP.md#resource-ownership-verification-p0406).
- [x] P04.07 Define consistent typed API errors and request IDs. See [error and correlation setup](../development/SETUP.md#typed-api-errors-and-request-ids-p0407), [ADR-019](../architecture/decisions/019-typed-api-errors.md) and the [completed execution record](completed/2026-10-27-api-errors.md): four-field safe errors and server-generated request IDs; own/independent verification and native review passed locally in `2340d2e`. [PR #59](https://github.com/cuenora/cuenora/pull/59) squash-integrated into `main` (`3bb2d07`); full published-tree equality verified locally.
- [x] P04.08 Add rate limits and protections to sensitive endpoints. Implemented and verified locally in `d1bb428`; own/independent checks passed and native review acknowledged. [PR #61](https://github.com/cuenora/cuenora/pull/61) merged into `main` (`a62e02e`); full published-tree equality verified. See the [completed execution record](completed/2026-10-27-auth-throttling.md).
- [x] P04.09 Test expired/revoked tokens and refresh-token reuse. Existing session adapter, refresh HTTP and identity middleware coverage passed against disposable PostgreSQL on 2026-10-28: 14 integration tests across three harness targets. No runtime changes were needed. See the [completed verification record](completed/2026-10-28-token-rejection-verification.md); [PR #63](https://github.com/cuenora/cuenora/pull/63) merged into `main` (`bcc9e8e`), with full published-tree equality verified.
- [x] P04.10 Verify user A cannot read or change user B's resources. Verified locally on 2026-11-02 across currently routed resources using the 20-target P04.06 PostgreSQL run and a focused negative-test audit. Extended existing transaction HTTP tests to reject foreign transfer destinations and replay attempts using another owner's existing edit/void keys, with unchanged financial snapshots; all ten mutation tests passed in the disposable harness (15.549 seconds). No runtime changes were needed. Future operations require their own negative authorization tests; the global read-only category catalog is not user-owned. See [cross-user verification](../development/SETUP.md#cross-user-resource-isolation-p0410).
- [x] P04.11 Implement logout and global session revocation. Implemented in `392cb27`, own/independent verification passed and native review acknowledged; [PR #65](https://github.com/cuenora/cuenora/pull/65) merged into `main` (`588d0c4`), with full published-tree equality verified. See [ADR-021](../architecture/decisions/021-session-logout.md) and the [completed execution record](completed/2026-10-28-session-logout.md).
- [x] P04.12 Design email verification and password recovery if passwords are owned. See the [design](../product/email-verification-and-recovery.md), [ADR-022](../architecture/decisions/022-email-verification-and-password-recovery.md) and [completed execution record](completed/2026-10-28-email-verification-recovery.md). [PR #67](https://github.com/cuenora/cuenora/pull/67) merged into `main` (`17c938a`), with full published-tree equality verified. Core policies are maintainer-approved; verification/recovery and browser login/verification are merged, with scoped proof in [the completed execution plan](completed/2026-11-03-web-identity.md). The design record alone is not runtime proof; use the subsequent completed runtime records below.

**Exit:** Secure, isolated accounts and negative authorization tests.

**Identity direction:** First-party email/password, open registration and verified
email before financial account creation or posting. Registration issues no session;
login/refresh alone grant no financial authority. P04.06/P04.10 cover all currently
routed resources; every future endpoint must preserve server-derived ownership.

**Runtime integration:** Session-bound verification and Resend delivery merged in
PR #105; terminal/browser password recovery in PR #106; memory-only browser login
and explicit verification in PR #107. See the completed
[verification](completed/2026-11-03-email-verification.md),
[recovery](completed/2026-11-03-password-recovery.md),
[browser recovery](completed/2026-11-03-recovery-web.md) and
[browser identity](completed/2026-11-03-web-identity.md) records. Provider configuration,
public HTTPS links and end-to-end staging proof remain deployment responsibilities.

### Phase 05 — Accounts and simple transactions (first vertical slice)

**Delivered entry prerequisites:** Verification/recovery runtime, confirmed profile
IANA zones, realized-date checks and resource ownership are implemented. Keep these
gates on future operations; there is no development bypass. Their implementation
and isolated verification are not evidence of production configuration.

- [x] P05.01 Model accounts and account types in the domain. Implemented and verified locally with five public-interface account tests; see the [account model](../product/accounts.md). API, storage integration and account operations remain later tasks.
- [x] P05.02 Implement account creation with the approved account/name model and currency, deriving ownership from authenticated identity and enforcing verified email; cover rejected attempts without writes. Verified locally in `13cf1cb` with seven isolated PostgreSQL HTTP cases, independent verification and acknowledged native review; see the [execution record](completed/2026-10-29-account-creation.md). Verification runtime and browser confirmation are now integrated; staging journey validation remains separate.
- [x] P05.03 Post a special dated opening-balance movement when nonzero, atomically with account creation; enforce confirmed-zone/date rules and safe retry behavior, never ordinary income. Implemented locally in `6804f91`, including explicitly approved minimal profile-zone confirmation, eight opening PostgreSQL cases, three profile cases and populated-key upgrade coverage; own/independent checks and native review acknowledged. See the [execution record](completed/2026-10-29-opening-balance.md). Verification runtime and browser confirmation are integrated; staging journey validation remains separate.
- [x] P05.04 Implement account listing scoped to the authenticated user, with cross-user isolation tests (P04.06/P04.10).
- [x] P05.05 Query account balances from append-only account movements, scoped to the authenticated owner; never accept a client-supplied balance or combine currencies.
- [x] P05.06 Implement account renaming with the approved name policy and archiving without deleting financial history; define archive behavior before implementation, preserving existing reference/type/currency constraints.
- [x] P05.07 Expose the existing global built-in income/expense category catalog read-only and validate category-kind compatibility; no custom-category creation or editing in the MVP. Implemented locally with authenticated reads available before email verification, isolated HTTP/storage checks and independent verification passed; native review acknowledged for `9dfe566`, integrated into local `main`. See the [catalog contract](../architecture/category-storage.md#read-only-http-catalog-p0507) and [ADR-028](../architecture/decisions/028-authenticated-category-catalog.md).
- [x] P05.08 Record income without an account. Implemented locally with exact money, verified-owner/zone gates, atomic minimal audit and permanent idempotency; own and independent checks passed for `9d62cb5`. The maintainer skipped native review for this candidate; future reviews remain enabled. Merged as [PR #78](https://github.com/cuenora/cuenora/pull/78); see the [execution plan](completed/2026-10-30-accountless-income.md).
- [x] P05.09 Record ordinary income linked to a normal account; reject savings destinations on creation, edits, and later assignment. Implemented locally in `a1bcfa0` with own/independent checks passed; native review skipped for this candidate by maintainer choice, future reviews remain enabled. [PR #79](https://github.com/cuenora/cuenora/pull/79) merged. Existing storage constraints reject savings updates/assignment; HTTP edit/void operations were subsequently implemented in P05.13. See the [execution plan](completed/2026-10-30-linked-income.md).
- [x] P05.10 Record an expense without an account. Implemented locally with independent permanent expense keys, verified-owner/date gates and atomic minimal audit, without movements. Implemented in `c3c8e1c`; own/independent checks passed. Native review skipped for this candidate by maintainer choice; future reviews remain enabled. [PR #80](https://github.com/cuenora/cuenora/pull/80) merged into `main`. See the [execution plan](completed/2026-10-30-accountless-expense.md).
- [x] P05.11 Record an expense linked to an account. Implemented locally with atomic negative posting to owned active normal/savings accounts, compatible permanent retries, and own/independent checks passed; native review skipped for this candidate by maintainer choice, as recorded in merged [PR #81](https://github.com/cuenora/cuenora/pull/81). See the [execution plan](completed/2026-10-30-linked-expense.md).
- [x] P05.12 Implement paginated transactions and date/category/account filters. Implemented locally for all five recorded kinds with exact amounts, verified-owner reads and combinable filters; own/independent checks passed, including 59 isolated infrastructure cases and baseline POST compatibility. Implemented in `1b1b526`; single-PR size exception approved; native review skipped for this candidate by maintainer choice, future reviews enabled. Merged as [PR #82](https://github.com/cuenora/cuenora/pull/82). See the [execution plan](completed/2026-10-30-transaction-listing.md).
- [x] P05.13 Implement atomic editing and voiding of transaction events with append-only compensating/new movements and minimal changed-field audit records; preserve accountless behavior, savings restrictions, ownership and exactly-once balance effects. Implemented locally for all five recorded kinds with linked inverse postings, permanent mutation retries and approved archive-correction policy. Own/independent checks passed, including 61 isolated PostgreSQL cases, ten expanded mutation groups and baseline HTTP compatibility. Single-PR size exception approved; native review skipped for this candidate by maintainer choice; future reviews remain enabled. Implemented in `60a5498` with harness integration `01adbec`; [PR #83](https://github.com/cuenora/cuenora/pull/83) merged into `main`. See the [execution plan](completed/2026-10-30-transaction-mutations.md).
- [x] P05.14 Add tests for balances, accountless entries, and rejection of ordinary income into savings on creation, edits, and reassignment. Existing balance/accountless/creation coverage is complemented by public HTTP rejection of linked normal-account income reassignment to savings, with and without an amount edit; rejected requests preserve financial snapshots and both balances. Workspace checks and isolated PostgreSQL acceptance targets passed locally; no production behavior changed. Commit/integrated into local `main`.

**Exit:** Register → log in → verify email → create account → confirm profile zone before dated posting → record income/expense → query correct balance via API. Nonzero opening creation also requires the zone. Prove rejected unverified/cross-user writes and retried postings leave no unauthorized or duplicate effects; never substitute test-seeded verification for the usable user journey.

### Phase 06 — Safe transfers

- [x] P06.01 Define the transfer contract: sent/received amounts and currencies, optional directional rate, precision/discrepancy rules, and rejection without partial effects. Accepted [contract and future acceptance cases](../product/currencies-and-transfers.md#recording-contract-p0601): positive exact optional rates with at most 12 fractional digits, half-even indicative amounts and informational discrepancy warnings that never override confirmed amounts. Contract-only task; no-quote creation and runtime test evidence are recorded below under P06.02/P06.03/P06.07. Optional rate runtime/storage was excluded from that contract-only task; local implementation is recorded under P06.06.
- [x] P06.02 Test that a transfer creates a source debit and destination credit with the correct amounts and currencies. The nine public HTTP currency-pair cases now reach and pass event, movement and account-balance assertions. The expanded sixteen-group transfer target also passes; ordinary Cargo ignores these database cases. See [run instructions](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607).
- [x] P06.03 Implement a single atomic SQL transaction for transfers. Implemented locally with event, debit, credit, minimal audit and permanent retry key in one transaction; own and independent checks passed, including 63 infrastructure groups and injected destination/audit/retry-insertion failures. Implemented in `f8a186f`; [PR #85](https://github.com/cuenora/cuenora/pull/85) merged into `main`; that implementation excludes quotes; local quote support is recorded under P06.06. See the [contract](../product/transfer-creation.md) and [execution plan](completed/2026-10-31-atomic-transfers.md).
- [x] P06.04 Block transfers involving another user's account. The owner-filtered account lookup already rejects a foreign source or destination with generic `404 not_found` before writing. Existing public HTTP regressions verify both sides and unchanged financial snapshots, including retry keys. Reverified on 2026-10-31 with the disposable PostgreSQL transfer target (all sixteen database groups passed); no runtime change was needed. See [acceptance tests](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607).
- [x] P06.05 Block same-source-and-destination transfers. Existing request validation compares normalized account UUIDs and rejects identical source/destination IDs with `400 invalid_request` before storage. The public HTTP regression verifies rejection and an unchanged financial snapshot, including retry keys. Reverified on 2026-10-31 with the disposable PostgreSQL transfer target (all sixteen database groups passed); no runtime change was needed. See [acceptance tests](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607).
- [x] P06.06 Support manual SEK/EUR/USD cross-currency transfers with required sent/received amounts and an optional rate; reject unsupported currencies. Implemented locally with canonical positive exact quotes, twelve-digit fractional precision, checked half-even informational comparisons, migration `0016` and backward-compatible no-quote retries. Own checks and independent S1–S4 verification passed, including 64 infrastructure groups and real pre-migration replay. Implemented in `7dd271a`; [PR #87](https://github.com/cuenora/cuenora/pull/87) is open against `main`, not merged. See the [execution plan](completed/2026-10-31-quoted-transfers.md) and [ADR-033](../architecture/decisions/033-exact-transfer-quotes.md).
- [x] P06.07 Implement `Idempotency-Key` for transfer POST requests. Independent permanent owner/operation-scoped keys, canonical request equivalence, conflicts, concurrent retries and original acknowledgements after edits/void/archive are locally verified. Migration `0015_transfer_creation_keys.sql` preserves existing financial records. Implemented in `f8a186f`; [PR #85](https://github.com/cuenora/cuenora/pull/85) merged into `main`; that commit accepts no quoted-rate fields; local quote support is recorded under P06.06.
- [x] P06.08 Force a failure between writes and verify full rollback. Existing public HTTP regressions inject a destination-movement failure after the source debit, plus later audit and retry-key failures; unchanged financial snapshots verify full rollback without consuming the retry key. Reverified on 2026-10-31 with the disposable PostgreSQL transfer target (all nineteen database groups passed) and both transfer migration-preservation targets. No runtime change was needed. See [acceptance tests](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607).
- [x] P06.09 Test retries, concurrency, and duplicate submissions. Existing public HTTP tests verify canonical replay without additional writes, conflicting input rejection, permanent replay after edits/void/archive, and concurrent equal-key submissions returning identical acknowledgements with exactly one event, two movements, one audit and one retry key. A distinct key creates a separate operation. Reverified on 2026-10-31 with the disposable PostgreSQL transfer target (all nineteen database groups passed); no runtime or test change was needed. See [acceptance tests](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607).
- [x] P06.10 Exclude transfers from income and expense statistics. Existing period-flow and monthly-flow queries admit only posted income, credited interest and expenses; category distribution admits only posted expenses. Reverified on 2026-11-02 with all three disposable PostgreSQL report targets (11 Rust HTTP tests; 36.800 seconds), including transfer fixtures and exact totals. No runtime or test changes were needed; transfers remain in account history and balances. See [verification instructions](../development/SETUP.md#transfer-exclusion-verification-p0610).

**Exit:** Transfers are atomic, secure, idempotent, and correctly reported.

### Phase 07 — Recurring expenses and due dates

- [x] P07.01 Define the policy for days 29/30/31 and February. Accepted last-valid-day adjustment for monthly, quarterly and annual schedules, preserving the original calendar anchor and nominal day without drift, including February 29. See the [calendar policy and acceptance examples](../product/recurrence-calendar.md) and [ADR-034](../architecture/decisions/034-anchored-recurrence-month-ends.md). Policy defined here; P07.04 implements the calculator; P07.05–P07.06 provide executable calendar regression coverage.
- [x] P07.02 Define the recurrence object, states, and lifecycle. See the [rule/occurrence model and transitions](../product/recurrence-lifecycle.md) and [ADR-035](../architecture/decisions/035-recurring-expense-lifecycle.md): preserve existing occurrences across pause/resume and edits, no paused-period backfill, stable scheduled-slot identity and explicit idempotent confirmation. Specification only; storage is delivered under P07.03; APIs, generation and executable lifecycle tests were subsequently delivered in P07.07–P07.12.
- [x] P07.03 Migrate `recurring_rules` and `recurring_occurrences`. Migration `0017` adds exact planning storage, owner/currency/category references, immutable scheduled slots, protected expense links and durable pause exclusions without posting financial effects. Own checks passed (67 infrastructure groups; Cargo format, Clippy and tests) and independent S1–S3 verification passed, including 45 rejected writes with unchanged public-table hashes. Implemented in `f1b1e79`; single size-exception PR approved, integrated into local `main`. See the [storage boundary](../architecture/recurrence-storage.md) and [execution record](completed/2026-10-31-recurrence-storage.md).
- [x] P07.04 Implement a pure next-occurrence calculator. `cuenora-domain` provides original-anchor weekly/monthly/quarterly/annual lookup after an optional exclusive civil date, without drift or financial effects; supported-year exhaustion returns no slot. Four basic API/boundary test groups, workspace format/Clippy/tests and isolated independent calendar/BusinessDate compatibility probes passed. Included in the P07.04–P07.06 delivery. Calendar regression coverage is extended under P07.05–P07.06. See the [calculator boundary](../product/recurrence-calendar.md#pure-calculator-p0704).
- [x] P07.05 Test weekly, monthly, quarterly, and annual recurrence. Eleven public-interface recurrence test groups cover weekly sequences, every date in RC01–RC13, original-anchor preservation, start/exclusive-search behavior and supported-year exhaustion. Isolated wrong-interval/drift mutations are rejected; the restored calculator passes without runtime changes. Included in the P07.04–P07.06 delivery. See [executable coverage](../product/recurrence-calendar.md#executable-calendar-coverage).
- [x] P07.06 Test leap years and year transitions. Recurrence regressions cover leap/non-leap February, Gregorian century rules (1900/2000/2100), return to February 29, December transitions for all four cadences and years 0001/9999. Included in the P07.04–P07.06 delivery. See [executable coverage](../product/recurrence-calendar.md#executable-calendar-coverage).
- [x] P07.07 Implement creating, editing, and pausing recurrence rules. Verified-owner create/PATCH/pause/resume APIs preserve existing occurrences and financial rows; permanent creation retries, future-only calendar-edit coverage, active-account selection and durable pause unions are enforced. Nine disposable PostgreSQL HTTP groups, retry migration/history constraints, the full 69-group infrastructure suite and independent S1–S4 probes passed. Local delivery; integrated into local `main`. See the [rule API](../product/recurrence-rules.md) and [execution record](completed/2026-11-01-recurrence-rules.md).
- [x] P07.08 Implement idempotent due-occurrence generation. Callable verified-owner generation recovers active backlog through rule-local today, honors pause/edit/end coverage and preserves existing snapshots without financial posting. Ten disposable PostgreSQL groups, the full 70-group infrastructure suite and independent lock/cancellation/compatibility probes passed. Implementation verified in `d9e6d32`; one maintainer-approved size-exception PR, integrated into local `main`. See the [generation contract](../product/recurrence-generation.md) and [execution record](completed/2026-11-01-due-occurrences.md).
- [x] P07.09 Add a safe daily worker/job with concurrency control. One-shot `cuenora-recurrence-job` supports external daily scheduling, database-scoped advisory exclusion on the same close-on-drop session as generation, bounded keyset discovery, per-rule rollback/continuation and privacy-safe exit/status output. Ten isolated PostgreSQL job groups, two CLI groups, existing recurrence regressions and independent S1–S4/session-loss/cancellation/HTTP differential checks passed. Implementation verified in `c9a4898`; one maintainer-approved size-exception PR against updated main, integrated into local `main`. See the [job contract](../product/recurrence-job.md) and [execution record](completed/2026-11-01-recurrence-job.md). Operator-reported restricted-role generation and the first automatic staging run are now recorded in the [Dokploy schedule guide](../development/DOKPLOY_RECURRENCE_JOB.md); continuous operation and production monitoring remain separate.
- [x] P07.10 Convert a confirmed occurrence into a posted transaction. Verified-owner actual-payment HTTP confirmation composes expense, optional movement, audit, occurrence link and permanent retry identity atomically. Thirteen isolated PostgreSQL confirmation groups, populated-history migration/constraint checks, the full 73-group infrastructure suite, workspace checks and independent S1–S2/cancellation/concurrency probes passed; 18 existing HTTP outcomes match the baseline. Implementation verified in `d4f1dfd`; [PR #93](https://github.com/cuenora/cuenora/pull/93) merged with maintainer-approved size exception, integrated into local `main`. See the [confirmation contract](../product/recurrence-confirmation.md) and [execution record](completed/2026-11-01-confirm-occurrence.md).
- [x] P07.11 Support skipping or postponing an occurrence. Verified locally: planning-only owner-scoped actions, strictly later due dates (including beyond rule end), permanent original-result retries and unchanged scheduled identity. Twelve isolated HTTP groups, populated-history migration checks, workspace checks, 12 baseline differential outcomes and independent S1/S2 probes passed. Full infrastructure run had one schema-catalog omission, corrected with a passing scoped recheck. Implementation `efd5446`; maintainer-approved size-exception [PR #94](https://github.com/cuenora/cuenora/pull/94) open, integrated into local `main`; see the [action contract](../product/recurrence-occurrence-actions.md) and [execution record](completed/2026-11-01-occurrence-actions.md).
- [x] P07.12 Prove future planned occurrences never change current balances. Verified locally with funded SEK/EUR/USD accounts, future linked/accountless fixtures, planning-action retries and an explicit actual-payment positive control. All 13 isolated occurrence-action groups, 10 generation groups and 9 account-listing groups passed, alongside workspace tests, Clippy and formatting. Test-only delivery; no runtime or migration changes. Integrated into local `main`; see the [regression proof](../product/recurrence-lifecycle.md#no-premature-accounting-regression-p0712).

**Exit:** Upcoming payment calendar with no premature accounting.

### Phase 08 — Interest-bearing savings

- [x] P08.01 Define optional interest terms, explicitly identified nominal annual rates, effective-dated changes, daily/monthly/quarterly/annual payouts, day count, balance timing, capitalization, and rounding before calculation. Maintainer-approved [interest policies](../product/savings.md#approved-forecast-policies-p0801) define closing daily balances, calendar-year day count, natural payout periods, period-end ties-to-even rounding, configurable forecast-only capitalization and inclusive rate changes. Policy-only task; calculator, storage and API were subsequently delivered in P08.02–P08.10.
- [x] P08.02 Migrate `savings_terms` and `savings_rate_periods`. Additive, immutable effective-date streams enforce savings-only ownership and exact nonnegative rates with at most six percentage fractional digits. Focused isolated PostgreSQL storage/upgrade/schema tests and independent S1–S2 probes passed. Integrated delivery; see [storage](../architecture/savings-storage.md) and [verification record](completed/2026-11-01-savings-storage.md).
- [x] P08.03 Allow savings-account interest terms to be configured. Verified-owner append/query APIs preserve independent immutable terms/rate histories, exact percentage strings and safe date-slot retries without financial effects. Disposable PostgreSQL HTTP tests, backend checks and independent security/concurrency verification passed; integrated into local `main`. See the [configuration contract](../architecture/savings-storage.md#configuration-api-p0803) and [execution record](completed/2026-11-01-savings-configuration.md).
- [x] P08.04 Implement a pure daily exact-decimal interest calculator. Verified locally: explicit nominal percentages, arbitrary-precision rational minor-unit accrual, Gregorian 365/366 day count and zero floor without daily rounding or financial effects. Six public API groups, workspace formatting/Clippy/tests and independent isolated numeric/dependency probes passed. No payout forecasts are included; see [calculator boundary](../product/savings.md#pure-daily-calculator-p0804) and [execution record](completed/2026-11-01-daily-interest.md).
- [x] P08.05 Implement forecasts for daily and monthly payouts with explicit annual-rate, day-count, capitalization, and rounding rules. Verified locally with P08.06: pure fixed-terms daily-input forecasts, exact accrual, natural period-end half-even rounding, partial first periods and unrounded open tails. Ten public-interface test groups, workspace tests, formatting, Clippy and repository checks passed; no endpoint or posting. Integrated delivery; see the [forecast boundary](../product/savings.md#pure-payout-forecasts-p0805-and-p0806).
- [x] P08.06 Support quarterly/annual payouts and configurable capitalization. Verified locally with P08.05: calendar-quarter/year ends and explicit exclusion/reinvestment; rounded projected payouts accrue from the following day with arbitrary-precision capital. Persisted terms-history orchestration remains outside this fixed-terms primitive. Integrated into local `main`; see the [forecast boundary](../product/savings.md#pure-payout-forecasts-p0805-and-p0806).
- [x] P08.07 Test uneven month lengths, leap years, and mid-period deposits/withdrawals. Verified locally: full 28/29/30/31-day monthly periods, constant-rate leap-year divisors, and explicit deposit/withdrawal closing balances with exact daily/monthly expectations and unchanged inputs. All 12 public forecast test groups, workspace tests, formatting, Clippy and repository checks passed; PostgreSQL tests remain ignored. Test/documentation-only delivery, no calculator or posting changes; integrated into local `main`. See [regression examples](../product/savings.md#calendar-and-balance-change-regressions-p0807).
- [x] P08.08 Specify whether negative rates are permitted; test zero rates, historical/future and mid-period rate changes, and the resulting acceptance/rejection policy. Existing approved policy permits zero and rejects negative nominal rates. Verified locally: explicit sign-policy tests, zero forecasts across all payout/capitalization modes, and historical/future daily-rate fixtures with inclusive mid-period changes without resetting monthly payouts. All 21 daily/forecast test groups, workspace tests, formatting, Clippy and repository checks passed; PostgreSQL tests were not rerun. No runtime changes in this regression slice; persisted-history orchestration is delivered in P08.09; integrated into local `main`. See [rate regressions](../product/savings.md#rate-policy-regressions-p0808).
- [x] P08.09 Add an explainable forecast endpoint with breakdown. Verified locally: verified-owner future forecasts use a consistent read-only posted snapshot, explicit flat-balance/partial-period assumptions, inclusive rates and exact period/payout breakdown. Unsupported payout/capitalization transitions and missing coverage reject explicitly. Eight isolated PostgreSQL HTTP groups (including concurrent writes), three existing configuration groups, workspace checks and independent S1–S3 security/snapshot probes passed. No postings; integrated into local `main`. See the [endpoint contract](../product/savings.md#explainable-forecast-api-p0809) and [execution record](completed/2026-11-02-savings-forecast-endpoint.md).
- [x] P08.10 Record actual interest exactly once in the receiving account, preserving the distinction from forecast interest and savings transfers. Verified locally: explicit `interest_income` creation accepts owned normal/savings receivers, posts one atomic credit, and preserves permanent retries and ordinary-income restrictions. Isolated PostgreSQL HTTP, workspace and independent security/concurrency/rollback/differential checks passed; no migration; integrated into local `main`. See the [execution record](completed/2026-11-02-actual-interest.md).
- [x] P08.11 Ensure forecast operations never affect posted balances. Verified locally: positive full-year forecasts across all four payout frequencies and both capitalization modes preserve public account balances, transaction history and stored financial snapshots after repeated/concurrent reads, with no actual-interest event. All nine isolated PostgreSQL forecast groups passed. Regression/documentation only; no production change or publication. See [isolation evidence](../product/savings.md#posted-balance-isolation-regression-p0811).
- [x] P08.12 Compare calculated outputs to hand-worked examples and document limitations. Verified locally: independent hand-worked expectations cover daily accrual, leap-year annual totals, inclusive 2%→3% mid-month rates, exact monthly half-even ties, capitalization and partial periods. All 23 daily/forecast test groups and workspace checks passed. Documented provider/day-count, historical/future input, rounding, terms-transition, range and reconciliation limits. Test/documentation-only delivery; integrated into local `main`. See [comparisons and limits](../product/savings.md#hand-worked-comparison-and-calculation-limits-p0812).

**Exit:** Transparent projections and correctly posted real payouts.

### Phase 09 — Insights and reporting

- [x] P09.01 Define dashboard metrics, selected-date pending-payment boundaries, and per-account planning difference; distinguish posted balance, net flow, savings, and estimates. Defined in the [metric specification](../product/dashboard-metrics.md): current balances, realized flows, savings subset, inclusive cutoff, separate persisted pending/future projections and per-account differences. Approved default: profile-zone month end. Metric-definition task; reporting runtime was subsequently delivered in P09.02–P09.05.
- [x] P09.02 Query aggregate income and expenses by period. Verified locally: `GET /v1/reports/flows` returns inclusive verified-owner income, expenses and net flow in exact SEK/EUR/USD rows, including accountless events and actual interest while excluding transfers/openings/voids. Three isolated PostgreSQL HTTP groups, sibling listing regression, workspace checks and independent ownership/read-only/correction probes passed. No migration; integrated into local `main`; see the [execution record](completed/2026-11-02-flow-reports.md).
- [x] P09.03 Query expense distribution by category. Verified locally: `GET /v1/reports/expense-distribution` returns exact per-currency category totals, explicit uncategorized bucket and two-place half-even percentages (null for zero spending), reconciling with P09.02. Four isolated PostgreSQL HTTP groups, flow/catalog regressions, workspace checks and independent ownership/correction/read-only/near-tie probes passed. No migration; integrated into local `main`; see the [execution record](completed/2026-11-02-expense-distribution.md).
- [x] P09.04 Query monthly net flow and history. Verified locally: `GET /v1/reports/monthly-flows` returns exact income, expenses and net flow per currency/month, zero-filled and clipped to inclusive selected dates, with a 120-calendar-month cap. Four isolated PostgreSQL HTTP groups, flow/distribution regressions, workspace checks and independent ownership/correction/read-only/time-zone/failure probes passed. No migration; integrated into local `main`; see the [execution record](completed/2026-11-02-monthly-flows.md).
- [x] P09.05 Query account balance history and implement per-account pending-payment totals and planning differences through a selected date, keeping unassigned occurrences visible. Verified locally: daily corrected-ledger history (366-day cap) and current per-account planning with separate pending/future projections, profile-zone month-end default, unassigned currency totals and null affected differences when due coverage is incomplete. Seven isolated PostgreSQL HTTP groups, workspace checks, five regression harnesses and independent correction/ownership/calendar/failure/no-write probes passed. No migration; integrated into local `main`; see the [execution record](completed/2026-11-02-account-planning.md).
- [x] P09.06 Separate currencies in reports without implicit FX. Existing P09.02–P09.05 behavior verified locally against disposable PostgreSQL; [verification record](completed/2026-11-02-report-currency-separation.md). Integrated into local `main`.
- [x] P09.07 Exclude transfers, account openings, and voided entries from normal income/spending. Existing report filters and public HTTP coverage verified locally against disposable PostgreSQL; [verification record](completed/2026-11-02-report-event-exclusions.md). Integrated into local `main`.
- [x] P09.08 Include accountless spending in financial reports. Existing period, category and monthly public HTTP coverage verified locally against disposable PostgreSQL, including unchanged storage after report reads; [verification record](completed/2026-11-02-accountless-report-spending.md). Integrated into local `main`.
- [x] P09.09 Test empty periods and pagination/filter combinations. Added empty-owner planning and exact transaction cursor-page membership with combined account/category/date filters and unchanged storage. Five isolated PostgreSQL targets (25 HTTP tests), workspace tests, formatting, Clippy and repository checks passed locally; [verification record](completed/2026-11-02-report-empty-periods-and-filters.md). No production behavior changes; integrated into local `main`.
- [x] P09.10 Profile and optimize queries using indexes and `EXPLAIN`. Added reproducible isolated profiling of seven production SQL statements across two owners and two ranges (28 plans; 20,200 transactions and 5,304 occurrences). Existing indexes retained: selective queries use them; no additional index or rewrite justified by this measured workload. Row counts, read-only snapshots and repository checks passed locally; [measurements and limitations](completed/2026-11-02-report-query-profiling.md). No migration or claimed speedup; integrated into local `main`.

**Exit:** Dashboard amounts reconcile with business transactions and account postings.

### Phase 10 — Shared mobile foundation (KMP)

**P10.12 CI status:** Both hosted shared-test jobs and their retained JUnit reports
were verified on 2026-11-08; see the [execution record](active/2026-11-07-shared-mobile-ci.md).

- [x] P10.01 Create KMP shared module and Android/iOS targets. Shared library, pinned wrapper/plugins and all three artifact builds verified locally and independently; see the [execution record](active/2026-11-06-kmp-foundation.md). Hosts/tests remain P10.02–P10.03; publication pending.
- [x] P10.02 Create minimal Android and iOS hosts that consume shared logic. Android debug APK/lint, unsigned iOS device/simulator Debug builds and independent shared-linkage verification passed; see the [execution record](active/2026-11-06-native-hosts.md). No runtime launch, backend integration or commonTest implied; native review disabled globally; publication pending.
- [x] P10.03 Set up `commonTest` and run a first test on both targets. On 2026-11-07, `SharedModuleTest.exposesSharedLibraryName` passed on Android JVM host and iOS ARM64 simulator (one test each, zero failures/errors/skips), locally and in an independent clean snapshot. `:shared:allTests`, all three shared artifact builds and repository checks passed; Android marker bytecode and both iOS exported headers match the baseline. See [shared test commands](../../mobile/README.md#shared-unit-tests). Non-fatal native debug-cache warnings remain; no UI/device/E2E or CI coverage implied. Native review disabled globally; publication pending.
- [x] P10.04 Model `Money`, `Account`, `Transaction`, and `SavingsForecast` in Kotlin. Pure shared domain models with no transport/runtime dependencies; recorded money and arbitrary-precision forecast values remain distinct. See [ADR-050](../architecture/decisions/050-shared-financial-domain-models.md). On 2026-11-07 all nine tests per Android-host/iOS-simulator target, `:shared:allTests`, all three shared artifact builds and repository checks passed locally. Serialization/client work remains P10.05–P10.06. Non-fatal native debug-cache warnings remain; publication pending.
- [x] P10.05 Serialize financial amounts as exact decimal strings. Adapter-only JSON serializers preserve recorded Money bounds and arbitrary-precision projections/report amounts. On 2026-11-07 all 14 tests per Android-host/iOS-simulator target, `:shared:allTests`, all three shared artifacts and repository checks passed, with independent baseline/candidate verification. See [ADR-051](../architecture/decisions/051-exact-json-amount-adapters.md) and [execution record](active/2026-11-07-decimal-serialization.md). Existing Xcode/debug-cache limitations remain; publication pending.
- [x] P10.06 Add Ktor HTTP client and typed API errors. Ktor 3.6.1 Android/Darwin transport preserves exact JSON and typed HTTP/transport failures without domain coupling, automatic redirects or application retries. On 2026-11-07 all 27 tests per Android-host/iOS-simulator target, `:shared:allTests`, all three shared artifacts and repository checks passed locally, with independent baseline/candidate verification including the actual timeout. See [ADR-052](../architecture/decisions/052-shared-http-client.md) and [execution record](active/2026-11-07-http-client.md). No remote backend/device-network check; existing Xcode/debug-cache limitations remain. Publication pending.
- [x] P10.07 Define `SessionStore` with native Keychain/Keystore implementations. Shared pair-level read/replace/clear with redacted failures, Android Keystore AES-GCM/no-backup storage and iOS device-only/non-sync Keychain. On 2026-11-07, 32 Android-host tests, 33 iOS-simulator tests, five isolated Android API37 instrumentation tests and six app-hosted iOS26.5 XCTest cases passed; allTests, AAR/two frameworks, repository checks and targeted independent verification passed. App-hosted tests exposed and pinned a CFBoolean bridge correction. See [ADR-053](../architecture/decisions/053-native-session-storage.md) and [execution record](active/2026-11-07-session-store.md). No login/refresh orchestration, physical-device, backup/restore or process-crash guarantee; existing native toolchain warnings remain. Publication pending.
- [x] P10.08 Provide account/transaction repositories with test fakes. Typed shared list/create/update accounts and list/create/edit/void transactions, exact amount/domain mapping, explicit session context and configurable commonTest fakes. On 2026-11-07, 58 Android-host and 59 iOS-simulator tests passed, plus allTests, AAR/two frameworks, repository checks and targeted independent recheck. See [ADR-054](../architecture/decisions/054-shared-resource-repositories.md) and [execution record](active/2026-11-07-repositories.md). Three independent response/currency defects were corrected and pinned. No auth orchestration, host wiring or live backend proof; existing Native warnings remain. Single future PR size exception approved; local work-unit commits recorded in the execution log, publication pending.
- [x] P10.09 Implement shared form validation. Pure opt-in account, transaction, authentication, recurrence and savings configuration validators with typed field/code results. On 2026-11-07, 73 Android-host and 74 iOS-simulator tests passed, plus allTests, AAR/two frameworks, repository checks and independent baseline/candidate verification. See [ADR-055](../architecture/decisions/055-shared-form-validation.md) and [execution record](active/2026-11-07-form-validation.md). IDNA, common-password checks, supported-zone lookup and server authority remain server-side; no UI or orchestration. Existing Native warnings remain. Single future PR size exception approved; publication pending.
- [ ] P10.10 Share presentation state logic where genuinely useful. Deferred by maintainer decision on 2026-11-07 until native product screens demonstrate genuinely shared presentation transitions; current hosts only display the shared marker.
- [x] P10.11 Test shared JSON contract fixtures in Rust and KMP. One synthetic corpus pins account listings, accountless income and quoted cross-currency transfers against actual Rust serializers and KMP HTTP repositories. Two Rust fixture tests, 76 Android-host and 77 iOS-simulator tests, workspace checks and all three shared artifacts passed locally on 2026-11-07. Independent scratch verification confirmed regeneration on fixture edits and production artifact isolation. No runtime/API changes or PostgreSQL/live HTTP proof; see the [execution record](active/2026-11-07-shared-json-fixtures.md) and [corpus guide](../../contracts/fixtures/README.md). Existing Native warnings remain; publication pending.
- [x] P10.12 Run shared tests for Android and iOS targets in CI. Hosted Android/Ubuntu and iOS ARM64 simulator/macOS jobs passed on 2026-11-08 after correcting the Android SDK package ID. Downloaded artifacts verified 76 Android-host and 77 iOS-simulator tests, zero failures/errors/skips; see the [CI run](https://github.com/cuenora/cuenora/actions/runs/42426340885).

**Exit:** Both apps compile with shared logic and unit tests.

### Phase 11 — Android native UI

- [x] P11.01 Build Compose navigation, theme, and base components. Locally implemented and verified on 2026-11-08: Android-specific Pen references, branded light/dark theme and bundled licensed fonts, adaptive drawer/bar/rail navigation and base controls. Thirteen API37 instrumented tests and independent app build/lint plus 76 shared host tests passed. Four lint warnings retained; combined lint/shared-test fixture dependency failure reproduced on baseline, separate commands pass. No functional financial/auth screens, API23 runtime or full accessibility proof; committed locally as `f40fe0b`, publication pending. See the [execution record](active/2026-11-08-android-native-foundation.md) and [native handoff](../product/android-foundation/README.md).
- [x] P11.02 Implement login/registration through shared repositories. Locally implemented and verified on 2026-11-08, including maintainer-approved session restoration, serialized renewal and sign-out. Shared tests 97 Android-host/98 iOS-simulator, nineteen API37 app tests and five Keystore tests passed; build/lint and independent security/concurrency probes passed. Four baseline lint warnings remain. Interrupted renewal requires a fresh sign-in to avoid refresh replay. An authorized Samsung API36 smoke against the configured backend also passed by maintainer-reported screens, including launch-time refresh and online/offline logout persistence; timed foreground renewal and API23 runtime remain unverified. One future PR size exception approved; publication pending. See the [execution record](active/2026-11-08-android-authentication.md).
- [x] P11.03 Implement onboarding to the first account. Locally implemented and independently verified on 2026-11-08, including maintainer-approved email verification, explicit profile-zone confirmation and optional dated opening balance. Added minimal self-only GET `/v1/me`, reused existing account adapters, and preserved uncertain writes in an encrypted owner/origin-bound journal. Shared suites 109 Android-host/110 iOS-simulator and 25 API37 native tests passed, as did app build/lint, format/Clippy, isolated profile/opening tests and the independent Rust workspace. Four baseline lint warnings and an independently reproduced baseline healthcheck flake remain documented. No hosted/mobile real-backend smoke, physical-device/API23 or full accessibility proof; backend deployment must include the new profile read. Commits `93b09ba`, `b806754`, `89a7084`; publication pending. See the [execution record](active/2026-11-08-android-onboarding.md).
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

- [ ] P12.01 Create the Xcode SwiftUI project linked to KMP shared logic. P10.02 supplies the baseline host; extend and verify that existing project rather than creating a competing app. This later phase task is not marked complete by the minimal scaffold alone.
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
- [x] P14.02 Build a non-root API container with health checks. PR #111 merged and alpha.5 was published for AMD64/ARM64. The operator reported the deployed container healthy; public checks verified readiness `200 → 503 → 200` across an operator-controlled PostgreSQL stop/start, with liveness 200 at the observed checks. Local isolated DB/probe/workspace tests and independent verification remain recorded in the [completed execution record](completed/2026-11-04-health-readiness.md); see [health guidance](../development/HEALTH_CHECKS.md) and [ADR-047](../architecture/decisions/047-process-health-and-schema-readiness.md). This does not certify exact Swarm settings, continuous uptime, traffic gating or backup restoration.
- [x] P14.03 Deploy PostgreSQL with least privilege and private networking. Verified for staging on 2026-11-06: restricted effective role/object permissions, actual deployed API connection as `cuenora_api`, private network controls and retained volume, earlier transfer/recovery/generation evidence, and operator-confirmed repeat email-verification, recurrence confirmation/actions and savings smokes after cutover. See the [runtime-role procedure and provenance](../development/POSTGRES_RUNTIME_ROLE.md#staging-completion-follow-up--2026-11-06). Repeat workflow outputs were not independently inspected; this is scoped staging completion, not production certification.
- [x] P14.04 Set up HTTPS, security headers, and reverse-proxy configuration. Verified for staging on 2026-11-06: public HTTPS/TLS 1.3, redirect and security-header checks; direct-origin HTTP/HTTPS rejection including spoofed headers; private network controls, exact immediate-proxy trust and independent client budgets/recovery. See the [operator evidence and maintenance guide](../development/STAGING_API_PROXY.md). Manual Traefik network/trust restoration after recreation is an accepted limitation; this does not certify production or complete Phase 14.
- [x] P14.05 Apply database migrations through a controlled procedure. Verified for the initial empty staging database: 23 pinned migrations, an empty-schema guard, `ON_ERROR_STOP` and one transaction; operator reported successful completion and the API authentication lookup then returned the expected 401 instead of 503. See the [initialization runbook](../development/INITIALIZE_ALPHA_DATABASE.md). Production/upgrade procedures and a migration-history runner remain separate, unverified work; this is not automatic startup migration.
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

Backend implementation for the recorded Phase 04–09 scope is integrated in local
`main`. Do not restart completed account, transaction, recurrence, interest or
reporting slices. Consult the completed records for exact acceptance boundaries.

1. Close the remaining staging deployment gaps: least-privilege PostgreSQL, private
   networking, origin ingress restrictions, immediate proxy trust and public-header
   spoofing/rate-budget checks. Container health and the observed readiness recovery
   are verified; they do not prove those infrastructure or traffic-admission boundaries.
2. Preserve the deployed recurrence schedule and extend its operational evidence:
   record the deployed image digest and monitor failed/missed and subsequent runs.
   Manual generation and the first automatic staging execution are operator-reported
   above; generation is not posting. Preserve the scoped identity/account/transfer
   checks rather than repeating synthetic writes blindly.
3. Establish encrypted off-server backups, prove restoration into staging, and add
   privacy-safe monitoring, alerts and rollback procedures. Initial schema bootstrap
   is verified; controlled upgrades/history and production migration remain open.
4. Add the missing security, load and recovery evidence (Phases 13–14), then assess
   readiness for controlled use. Four concurrent transfers are not a load test.
5. Build KMP and native Android/iOS vertical slices against implemented contracts
   (Phases 10–12), with platform/shared E2E checks (Phase 13). Terminal helpers do
   not complete shared Maestro or native-client tasks. Phase 15 remains post-MVP.

These are dependency boundaries, not authorization for new remote operations or
an entire phase. Phase 14 remains open; checked bootstrap delivery and a published
alpha do not establish production readiness.

### Accepted policies and continuing limits

- Operation-specific idempotency, account archive rules, recurrence month ends and
  postponement are documented and implemented; retain their original contracts.
- Savings forecasts use explicit exact-money/day-count/rounding rules and reject
  unsupported terms transitions or missing coverage. They never post money.
- Account planning separates posted balances from persisted pending occurrences
  and future projections. It is not bank availability or permission to spend.
- New product policies require explicit decisions; do not reinterpret implementation
  omissions or provider-specific behavior as approved defaults.

## 14. Technical references

- Kotlin Multiplatform native UI/shared logic: https://kotlinlang.org/docs/tutorials/native/mpp-ios-android.html
- Kotlin Multiplatform tests: https://kotlinlang.org/docs/multiplatform/multiplatform-run-tests.html
- Maestro: https://maestro.dev/
- Axum: https://docs.rs/axum/latest/axum/
- SQLx: https://docs.rs/sqlx/latest/sqlx/
- PostgreSQL numeric types: https://www.postgresql.org/docs/current/datatype-numeric.html
- OWASP MASVS: https://mas.owasp.org/MASVS/

## Next planning artifact

Scope the next remaining Phase 14 staging-readiness gap using [TEMPLATE.md](TEMPLATE.md).
Start from the published alpha and verified initial schema rather than repeating
container publication or the empty-database bootstrap. Record precise acceptance
for health monitoring, infrastructure boundaries, upgrades, recovery and rollback.
Keep production, mobile and backend implementation status separate.

Use the [plans index](README.md) for completed implementation records. Preserve
recorded test limitations and outstanding operational work when planning release.
