# Cuenora

**Open-source personal finance, clearly organized.**

Cuenora is a non-commercial, open-source personal finance project designed to help people understand and organize their money. It aims to provide a single place to track accounts, income, expenses, transfers, recurring payments, and interest-bearing savings, with clear statistics and insights.

> **Project status:** Early development. The API provides local liveness, pending registration, login and refresh rotation; financial features and mobile applications are not implemented yet. Cuenora is not ready for personal financial data or production use.

## Vision

Make everyday financial tracking straightforward, transparent, and accessible through native mobile experiences, a well-defined API, and a self-hostable backend.

Cuenora is intended to be a personal, community-friendly software project. It is not a bank, does not hold funds, and is not intended to provide financial advice.

See the [design guide](DESIGN.md) for the visual direction, accessibility rules,
financial presentation principles and editable design references.

## Planned features

- **Multiple accounts:** Organize bank accounts, cash, and other balances in one place.
- **Income and expenses:** Record amounts, dates, descriptions, and categories. Allow transactions to be recorded without assigning them to an account.
- **Transfers:** Move money between owned accounts without treating the transfer as income or an expense.
- **Recurring payments:** Define schedules for recurring expenses and track when they are actually paid.
- **Interest-bearing savings:** Record savings balances, interest rates, and payout schedules. Keep projected interest separate from credited interest.
- **Statistics:** Explore balances, cash flow, spending by category, and trends over time.
- **Privacy and control:** Store data in PostgreSQL behind a Rust API, with an emphasis on secure account isolation and data portability.

These are **planned capabilities**, not a list of completed features. The scope and implementation details may change as the project develops.

## Planned technology stack

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

The following structure is **planned** and will be introduced incrementally:

```text
cuenora/
├── .github/        # CI workflows and community configuration
├── backend/        # Rust API, domain logic, and database migrations
├── mobile/         # Android, iOS, and shared Kotlin Multiplatform code
├── contracts/      # API contract and compatibility checks
├── tests/          # Cross-platform end-to-end tests and shared fixtures
├── infra/          # Local development and deployment infrastructure
├── docs/           # Architecture, decisions, and development guides
├── .editorconfig
├── .gitignore
├── LICENSE
└── README.md
```

## Development

The development environment and commands will be documented as each component is initialized. The expected prerequisites are:

- Rust and Cargo for the backend.
- PostgreSQL, typically started with Docker Compose for local development.
- A compatible JDK and Android SDK for Kotlin Multiplatform and Android development.
- macOS and Xcode for building and testing the iOS application.

The backend Cargo workspace builds with the toolchain pinned in
`backend/rust-toolchain.toml`. It contains `cuenora-domain` (exact `Money` and
`Currency` value types) and `cuenora-api` (a minimal HTTP server with
`GET /health/live`, `POST /auth/register`, `POST /auth/login` and `POST /auth/refresh`). Registration
stores an unverified identity and salted Argon2id credential using an explicitly configured database;
valid duplicates receive the same response without replacing credentials. See
[registration setup](docs/development/SETUP.md#pending-user-registration-p0402).
Financial application use cases and posting are not implemented yet. See the [money specification](docs/product/money.md)
for the implemented domain rules.
User, credential and opaque access/refresh-session migrations are available with
isolated PostgreSQL tests; see [identity storage](docs/architecture/identity-storage.md).
Login verifies exact password bytes and atomically issues opaque access/refresh
credentials, persisting only digests. Access lasts 15 minutes and refresh lasts
30 days; pending users receive no financial authority. See
[login setup](docs/development/SETUP.md#opaque-login-p0403). Refresh rotates both
secrets and extends refresh validity for 30 days without an absolute session-age
cap; consumed-token reuse revokes its family. See
[refresh setup](docs/development/SETUP.md#sliding-refresh-rotation-p0404).
Reusable route-scoped bearer identity middleware validates access expiry and
family revocation; global logout is the first protected production endpoint. See
[middleware integration](docs/development/SETUP.md#authenticated-identity-middleware-p0405).
Errors use typed `code`, `message`, `details` and `request_id` fields, with a
server-generated `X-Request-ID` on responses; see
[error and correlation setup](docs/development/SETUP.md#typed-api-errors-and-request-ids-p0407).
Authentication attempt limits and explicit proxy trust are implemented in P04.08; see the
[completed execution record](docs/plans/completed/2026-10-27-auth-throttling.md) and
[deployment prerequisites](docs/development/SETUP.md#authentication-abuse-protection-p0408).
Logout/global revocation is implemented in P04.11; see [logout setup](docs/development/SETUP.md#logout-and-global-session-revocation-p0411).
Email verification and financial authorization remain pending.
Account migrations also provide supported types/currencies and ownership
references; see [account storage](docs/architecture/account-storage.md).
The shared read-only built-in category catalog is described in
[category storage](docs/architecture/category-storage.md); category APIs remain pending.
Transaction event storage is described in [transaction storage](docs/architecture/transaction-storage.md).
Append-only postings are described in [account movement storage](docs/architecture/account-movement-storage.md).
Minimal append-only audit metadata is described in
[transaction audit storage](docs/architecture/transaction-audit-storage.md).
Cross-table currency and savings guards are described in
[financial storage constraints](docs/architecture/financial-constraints.md).
Posting workflows, automatic auditing and transaction APIs remain pending.
See [development setup](docs/development/SETUP.md) for prerequisites and checks.

```bash
cd backend
cargo run -p cuenora-api --locked
```

Check liveness at `http://127.0.0.1:3000/health/live` (HTTP 200, body `OK`).
The [OpenAPI contract](contracts/openapi.yaml) describes the implemented endpoint.

## Testing strategy

Cuenora aims to make correctness a first-class requirement, especially for monetary calculations and transfers.

- **Backend unit tests:** Financial domain rules and application logic in Rust.
- **Backend integration tests:** PostgreSQL queries, transactions, and API behavior.
- **Shared mobile unit tests:** Models, validation, and shared presentation logic in Kotlin Multiplatform `commonTest`.
- **Platform-specific tests:** Android and iOS integrations, native behavior, and user interfaces.
- **Mobile end-to-end tests:** Shared Maestro flows wherever behavior is consistent across Android and iOS.

All money-related logic should use exact decimal or integer representations rather than binary floating-point arithmetic.

## Contributing

Contributions, ideas, and constructive feedback are welcome. During the setup phase, please open an issue to discuss substantial changes before starting a large pull request.

Contribution guidelines and automated checks will be added as the repository evolves.

## Security and privacy

Personal financial information is sensitive. Security, input validation, authorization, and isolation between users are core requirements, not optional enhancements.

Please **do not publish sensitive vulnerabilities or real financial data in public issues**. If GitHub private vulnerability reporting is enabled for this repository, use that channel instead. A dedicated security policy will be added as part of the initial project setup.

## License

Cuenora is licensed under the [MIT License](LICENSE).

The project is developed without commercial goals, but the MIT License permits commercial reuse by others under its terms.
