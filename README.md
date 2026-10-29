# Cuenora

**Open-source personal finance, clearly organized.**

Cuenora is a non-commercial, open-source personal finance project designed to help people understand and organize their money. It aims to provide a single place to track accounts, income, expenses, transfers, recurring payments, and interest-bearing savings, with clear statistics and insights.

> **Project status:** Early development. Money/account domain models, financial storage and authentication/session APIs exist. Account creation, dated openings and profile-zone confirmation are merged. Verified-owner metadata listing is implemented locally, with own/independent checks passed and final review pending. Email verification/recovery, remaining financial workflows and mobile applications are pending. Cuenora is not ready for personal financial data or production use. See the [current delivery snapshot](docs/plans/master-plan.md#current-delivery-snapshot).

## Vision

Make everyday financial tracking straightforward, transparent, and accessible through native mobile experiences, a well-defined API, and a self-hostable backend.

Cuenora is intended to be a personal, community-friendly software project. It is not a bank, does not hold funds, and is not intended to provide financial advice.

See the [design guide](DESIGN.md) for the visual direction, accessibility rules,
financial presentation principles and editable design references.

The first goal is to see each account's recorded balance alongside its upcoming
payments, so a shortfall in one account is visible even when money exists elsewhere.
Records are maintained manually: Cuenora does not read live bank balances, move
money or promise that a planning estimate is safe to spend.

## Planned features

- **Multiple accounts:** Organize bank accounts, cash, and other balances in one place.
- **Income and expenses:** Record amounts, dates, descriptions, and categories. Allow transactions to be recorded without assigning them to an account.
- **Transfers:** Move money between owned accounts without treating the transfer as income or an expense.
- **Recurring payments:** Define schedules for recurring expenses and track when they are actually paid.
- **Interest-bearing savings:** Record savings balances, interest rates, and payout schedules. Keep projected interest separate from credited interest.
- **Statistics:** Compare pending payments with each account's recorded balance; explore cash flow, built-in spending categories and trends, without combining currencies implicitly.
- **Privacy and control:** Store data in PostgreSQL behind a Rust API, with an emphasis on secure account isolation and data portability.

These are **planned capabilities**, not a list of completed features. The scope and implementation details may change as the project develops.

## Technology direction

| Area | Technology |
| --- | --- |
| Backend API | Rust, Axum, Tokio |
| Database | PostgreSQL |
| Data access | SQLx |
| API contract | OpenAPI |
| Shared mobile logic | Kotlin Multiplatform |
| Android UI | Kotlin, Jetpack Compose |
| iOS UI | Swift, SwiftUI |
| Shared mobile unit tests | Kotlin Multiplatform `commonTest` |
| Mobile end-to-end tests | Maestro |
| Local infrastructure | Docker Compose |
| Continuous integration | GitHub Actions |

The backend, mobile clients, and infrastructure will be independently buildable within a single repository.
See [ADR-001](docs/architecture/decisions/001-initial-stack.md) for the chosen
direction, tradeoffs, and implementation status.

## Repository layout

Current development surfaces (mobile projects are not initialized yet):

```text
cuenora/
├── .github/        # CI workflows and community configuration
├── backend/        # Rust API, domain logic, and database migrations
├── contracts/      # API contract and compatibility checks
├── tests/          # Isolated PostgreSQL/infrastructure acceptance harness
├── infra/          # Local PostgreSQL infrastructure
├── scripts/        # Repository checks
├── docs/           # Architecture, decisions, and development guides
├── .editorconfig
├── .gitignore
├── LICENSE
└── README.md
```

## Development

Start with the [setup guide](docs/development/SETUP.md) for the pinned Rust
compiler, local PostgreSQL, isolated tests and current CI commands. Mobile SDKs
and macOS/Xcode will be needed when the Android/iOS projects are introduced;
they are not prerequisites for working on today's backend.

| Implemented foundation | Reference and remaining boundary |
| --- | --- |
| Exact money and account metadata in `cuenora-domain` | [Money](docs/product/money.md), [accounts](docs/product/accounts.md); no balance/posting use cases |
| Registration, login, refresh, logout and global revocation in `cuenora-api` | [Setup](docs/development/SETUP.md), [OpenAPI](contracts/openapi.yaml); authentication is not financial authorization |
| Route-scoped identity, typed errors and bounded authentication throttling | [Identity strategy](docs/product/identity-and-access.md); account creation enforces verified ownership; other resource gates remain pending |
| Verified-owner account creation with lifetime idempotency | [Creation contract](docs/product/account-creation.md); optional opening posting verified locally; no usable email-verification journey yet |
| Identity and financial storage, global read-only categories, append-only postings/audit | [Documentation map](docs/README.md); constraints do not implement transactional application workflows |
| Email verification and password recovery design | [Approved design](docs/product/email-verification-and-recovery.md); no challenge/delivery/reset runtime yet |

The API's implemented paths are `/v1/health/live` and `/v1/auth/register`, `/v1/auth/login`,
`/v1/auth/refresh`, `/v1/auth/logout`, `/v1/auth/logout-all`, plus `POST /v1/accounts`.
P05.03 adds `PATCH /v1/me` for explicit zone confirmation and optional dated
[opening balances](docs/product/opening-balances.md) locally; own/independent checks and native review passed; publication pending.
All use `/v1`; unversioned paths return 404 without aliases. P05.04 adds
`GET /v1/accounts` locally with own/independent checks passed; native review pending; see the
[metadata listing contract](docs/product/account-listing.md).
Start the local API with:

```bash
cd backend
cargo run -p cuenora-api --locked
```

Check liveness at `http://127.0.0.1:3000/v1/health/live` (HTTP 200, body `OK`).
Liveness does not prove database readiness or financial workflow availability.
The [OpenAPI contract](contracts/openapi.yaml) describes the implemented routes.

## Testing strategy

Cuenora aims to make correctness a first-class requirement, especially for monetary calculations and transfers.

- **Backend unit tests:** Financial domain rules and application logic in Rust.
- **Backend integration tests:** PostgreSQL queries, transactions, and API behavior.
- **Shared mobile unit tests:** Models, validation, and shared presentation logic in Kotlin Multiplatform `commonTest`.
- **Platform-specific tests:** Android and iOS integrations, native behavior, and user interfaces.
- **Mobile end-to-end tests:** Shared Maestro flows wherever behavior is consistent across Android and iOS.

All money-related logic should use exact decimal or integer representations rather than binary floating-point arithmetic.

## Contributing

Follow [CONTRIBUTING.md](CONTRIBUTING.md) and the [documentation map](docs/README.md).
Choose one focused task from the [current implementation order](docs/plans/master-plan.md#13-current-implementation-order),
confirm its prerequisites and add tests/docs with the behavior. Discuss substantial
changes before starting a large pull request; a roadmap entry alone is not permission
to expand scope.

## Security and privacy

Personal financial information is sensitive. Security, input validation, authorization, and isolation between users are core requirements, not optional enhancements.

Please **do not publish sensitive vulnerabilities or real financial data in public issues**.
Follow [SECURITY.md](SECURITY.md) for private reporting and the early-development
support boundary.

## License

Cuenora is licensed under the [MIT License](LICENSE).

The project is developed without commercial goals, but the MIT License permits commercial reuse by others under its terms.
