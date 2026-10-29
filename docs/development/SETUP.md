# Development environment

The Rust backend workspace contains two crates:
`cuenora-domain` at `backend/crates/domain` and `cuenora-api` at
`backend/crates/api`. The domain library uses Chrono for strict Gregorian dates and implements
`Money`, `Currency`, typed monetary errors and account metadata; see the
[money specification](../product/money.md) and [account model](../product/accounts.md).
The API uses Axum, Tokio and SQLx for local liveness, pending registration and
public login and refresh rotation, plus reusable bearer identity middleware.
Account creation is implemented locally; own/independent checks and native review passed; not yet published. See the
[creation contract](../product/account-creation.md) for required verified-email
authorization and lifetime idempotency. Posting workflows are not implemented. Global logout is a protected production route.
Refresh reuse revokes its owning session;
logout/global revocation is available in P04.11 below.

## Prerequisites

- Git
- Python 3 (for lightweight repository checks)
- Docker and a Dev Containers-compatible editor, **or** a local Rust toolchain

The Dev Container is intended for **Rust backend development and repository tooling**.
It does not install Android SDK or Xcode. Android and iOS development environments
will be configured when the mobile project is initialized.

## Option A: Dev Container (recommended for backend development)

1. Clone the repository:

   ```bash
   git clone https://github.com/cuenora/cuenora.git
   cd cuenora
   ```

2. Install Docker and a Dev Containers-compatible editor.
3. Open this repository in the editor.
4. Choose **Dev Containers: Reopen in Container**.
5. Verify the toolchain and repository checks in the container:

   ```bash
   cd backend
   rustup show
   rustc --version
   cargo --version
   cargo clippy --version
   cargo fmt --version
   cd ..
   python3 scripts/check_repository.py
   ```

The development container image is based on Debian Bookworm and contains Rust
utilities. The Rust toolchain is pinned to `1.99.1` in
`backend/rust-toolchain.toml`, including Rustfmt and Clippy. Run Rust commands
from `backend/` so rustup selects this toolchain. Ensure local version managers
or `RUSTUP_TOOLCHAIN` overrides do not select a different compiler.

## Option B: Local installation

Install Rust through [rustup](https://rustup.rs/). From the repository root:

```bash
cd backend
rustup show
cargo clippy --version
cargo fmt --version
cd ..
python3 scripts/check_repository.py
```

From `backend/`, validate and build the workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --locked
```

The API has an HTTP-level liveness route test. The domain has 20 public-interface
test groups for exact parsing, arithmetic, bounds and half-even quantization.
Financial application use cases remain pending; registration database cases run
explicitly through the disposable infrastructure harness described below.
Run just the monetary acceptance tests from `backend/` with
`cargo test -p cuenora-domain --test money --locked`. If an ambient override selects
another compiler, prefix Cargo commands with `rustup run 1.99.1` to use the pin.

## Run the local API

From `backend/`, start the server:

```bash
cargo run -p cuenora-api --locked
```

It listens only on `127.0.0.1:3000` and prints its local address after binding.
From another terminal, check the process:

```bash
curl --fail --include http://127.0.0.1:3000/v1/health/live
```

Expect status `200`, content type `text/plain; charset=utf-8`, and body `OK`.
The response has no trailing newline. PostgreSQL, `.env`, and credentials are not
required; liveness does not check database readiness. No readiness route exists.
If port 3000 is occupied, startup exits with an error; stop the conflicting local
process before retrying. Stop the API with Ctrl+C.

The endpoint is described in [`contracts/openapi.yaml`](../../contracts/openapi.yaml).
The local-only address is fixed for this first endpoint; deployment and runtime
configuration are separate tasks.

## Rust formatting and lint conventions

Use the Rustfmt and Clippy versions shipped with the pinned backend toolchain.
Run these commands from `backend/`.

To apply formatting locally:

```bash
cargo fmt --all
```

Use `cargo fmt --all -- --check` to validate without modifying files, as CI does.
Use Rustfmt defaults; no custom `rustfmt.toml` is needed for the current workspace.

Run Clippy for the entire workspace with the same scope and warning policy as CI:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Fix warnings before submitting a PR. Avoid blanket lint suppressions and introduce
custom lint groups only when a concrete project requirement justifies them.

A task runner (`make` or `just`) is deferred: the current workflow is a short list
of direct commands and does not need additional orchestration. Keeping Cargo
commands explicit avoids another prerequisite and duplicate command definitions.
Revisit this decision when repeated multi-step tasks justify a shared entry point.

## Local PostgreSQL

Install Docker Engine or Docker Desktop with Docker Compose v2 or later. Run the
following from the repository root against your local Docker daemon; the Dev
Container does not configure Docker access automatically.

The development-only service uses the official `postgres:18-bookworm` image,
a project-scoped named volume, and a host port bound only to `127.0.0.1`.
Registration can use the identity schema after migrations 0001–0008 are applied.
The disposable infrastructure suite also applies `0009_session_access_tokens.sql`.
The server does not apply migrations automatically. Use synthetic data only. The `cuenora` bootstrap role is a local database administrator, not a
production application role.

Access rows persist unique 32-byte digests bound to the matching session/user
pair. Credential lookup and initial issuance compare a server-owned credential
snapshot under a row lock and atomically create the session/access/refresh
family. `POST /v1/auth/login` returns the opaque token pair once with `Cache-Control:
no-store`; the database cannot verify SHA-256 provenance. The P04.05 middleware
performs access digest lookup, expiry and owning-session revocation checks when
explicitly applied to a protected route.

In Bash, generate a password for a new database without printing it:

```bash
export CUENORA_POSTGRES_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
docker compose -f infra/compose.yaml config --quiet
docker compose -f infra/compose.yaml up -d --wait --wait-timeout 90
docker compose -f infra/compose.yaml ps
```

Keep the password in your current shell, or store it in a local secret manager
and restore it for later sessions. Do not regenerate it when reusing an existing
volume: image initialization variables only apply to an empty data directory.
Never commit passwords or print rendered Compose configuration; use
`config --quiet` for validation.

### Optional local environment file

[`.env.example`](../../.env.example) lists the variables currently supported by
Compose. Its password is intentionally empty: the untouched example cannot start
a database unless a password is supplied from the shell. Application database
configuration is exported separately; the API never loads `.env`.

From the repository root, copy the template only if `.env` does not already exist:

```bash
(umask 077; set -o noclobber; cat .env.example > .env)
```

This Bash command refuses to overwrite an existing file and creates a private
copy readable only by your user. Edit `.env` locally and fill
`CUENORA_POSTGRES_PASSWORD`; keep or change `CUENORA_POSTGRES_PORT` as needed.
For a new database, use a generated password from a password manager. For an
existing database volume, use its original password rather than generating a new
one. Keep the file private; `.env` is ignored by Git, while `.env.example` is
tracked without credentials. Never paste passwords into documentation or PRs.

Load the file explicitly with every Compose command in this mode:

```bash
docker compose --env-file .env -f infra/compose.yaml config --quiet
docker compose --env-file .env -f infra/compose.yaml up -d --wait --wait-timeout 90
docker compose --env-file .env -f infra/compose.yaml ps
docker compose --env-file .env -f infra/compose.yaml exec postgres \
  psql -h 127.0.0.1 -U cuenora -d cuenora -W
docker compose --env-file .env -f infra/compose.yaml down
```

Exported shell variables take precedence over `.env` entries. If you previously
used the shell-based setup and now want the file's values, first run:

```bash
unset CUENORA_POSTGRES_PASSWORD CUENORA_POSTGRES_PORT
```

Use `config --quiet`; plain `config` renders the password. The original
shell-export workflow above still works without an env file.

### Connection and lifecycle

The database and user are both `cuenora`, and the default connection is
`127.0.0.1:5432`. To use a different port, set `CUENORA_POSTGRES_PORT` before
starting the service. Connect over TCP using the container's bundled client and
enter the password at its prompt:

```bash
docker compose -f infra/compose.yaml exec postgres \
  psql -h 127.0.0.1 -U cuenora -d cuenora -W
```

Stop containers and the network while preserving the database volume:

```bash
docker compose -f infra/compose.yaml down
```

Only when intentionally discarding this project's local database, remove its
volume too:

```bash
docker compose -f infra/compose.yaml down --volumes
```

Compose defaults to project `cuenora-local`. For separate checkouts or disposable
environments, use a unique `-p <project-name>` with every Compose command and a
different `CUENORA_POSTGRES_PORT`. Never point integration tests at the default
development database. The isolated smoke test chooses fresh projects and free
ports, checks password rejection and persistence, and removes its own volumes:

```bash
python3 tests/infrastructure/test_postgres_compose.py
```

### Sliding refresh rotation (P04.04)

The `session_store` adapter rotates one-time opaque refresh credentials, extending
refresh validity for 30 days after each successful rotation with no absolute
family-age cap. Consumed-token reuse revokes only that session family. Explicit
family revocation is possession-scoped and idempotent. Secrets are never stored;
the existing session tables retain digests and consumed generations. See
[ADR-016](../architecture/decisions/016-sliding-refresh-rotation.md) for races,
lost responses and the sliding-expiry tradeoff. `POST /v1/auth/refresh` accepts JSON
with only a `refresh_token` string (8 KiB maximum body), returning the same five
token-pair fields as login. Access lasts 900 seconds and successor refresh lasts
2592000 seconds after each rotation. Ordinary API activity alone does not renew
refresh validity. Logout/global-revocation routes are described in P04.11 below; reusable bearer
middleware is described below.

All refresh responses are JSON with `Cache-Control: no-store`, without cookies.
Malformed JSON/fields return 400, oversized bodies 413, unsupported/missing JSON
content type 415, with `code: invalid_request`. Invalid, expired, revoked
or consumed credentials return 401 `code: invalid_refresh_token`; consumed
reuse commits family revocation before rejection. Missing storage, storage/entropy
failures or full separate two-slot refresh capacity return 503
`code: refresh_unavailable`. Existing registration/login hashing capacity
is not consumed by refresh. See [ADR-017](../architecture/decisions/017-refresh-http.md).

Serialize refresh requests and replace both client-held secrets together. Retrying
after a lost successful response or racing another refresh can revoke the family
and require login; a 503 around commit may also be ambiguous. No retry grace is
implemented. A session can live indefinitely through successful rotations; after
30 days without a valid refresh, a fresh login is required. Pending users can
refresh without verification or financial authority.

Run the six adapter acceptance groups using disposable local PostgreSQL:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_session_store_against_disposable_postgres \
  PostgresComposeTest.test_refresh_http_against_disposable_postgres -v
```

The harness asserts all six adapter groups and five HTTP groups run. They cover
sliding lifetimes, family-specific replay/revocation, invalid/expired credentials,
atomic failures, concurrent operations, cancellation and separate HTTP admission.
Ordinary Cargo tests exercise two transport cases but intentionally ignore the DB
groups. The new focused target can be selected alone with
`PostgresComposeTest.test_refresh_http_against_disposable_postgres`.

### Typed API errors and request IDs (P04.07)

Application errors now use four keys, replacing the old `{error: ...}` shape:

```json
{"code":"invalid_request","message":"The request is invalid.","details":null,"request_id":"0123456789abcdef0123456789abcdef"}
```

Branch on `code`, not English `message`. Existing codes/statuses, success bodies,
Bearer challenges and route-specific `Cache-Control: no-store` remain unchanged.
`details` is currently always null: submitted values, tokens and backend diagnostics
are never included. Unknown paths return 404 `not_found`; unsupported methods return 405
`method_not_allowed` with the existing `Allow` header. HEAD responses omit bodies.

The server generates 128 random bits as 32 lowercase hexadecimal characters for
`X-Request-ID` on successes and errors; the error's `request_id` matches it. IDs
are diagnostics only, not credentials, ownership proof or retry/idempotency keys.
Supplied ID headers (including duplicates) are replaced, not trusted or reflected.
An unavailable OS entropy source rejects before the handler with 503
`request_id_unavailable`, `details: null`, `request_id: null`, `Cache-Control: no-store`
and no `X-Request-ID`. No fake ID is returned and no application write is attempted.

`app(pool)` already has context middleware. When merging additional routes, wrap
**the final router** with `cuenora_api::request_id::with_request_context(router)`.
Nested wrappers reuse the same typed server extension, not another random ID.
Handlers extract `Extension<RequestId>` and render `ApiError::... .response(&id)`;
private fields prevent client IDs from becoming trusted context. Authentication
middleware also establishes context when used standalone. Context is not a panic
catcher, proxy request logger or a replacement for typed extractor rejection handling.
HTTP parse failures rejected before Axum and broken connections are outside this
application-response contract.

From `backend/`, run `cargo test -p cuenora-api --test api_errors --locked`.
The existing disposable HTTP suites also check the new envelope/ID correlation.
See [ADR-019](../architecture/decisions/019-typed-api-errors.md) and the
[OpenAPI contract](../../contracts/openapi.yaml) for codes and response schemas.

### Authenticated identity middleware (P04.05)

`cuenora_api::auth_identity::require_identity` is reusable middleware, not a new
endpoint. Apply `axum::middleware::from_fn_with_state(pool, require_identity)`
with `.route_layer(...)` to protected routes; see the compiling integration
example on that function. Handlers extract `Extension<AuthenticatedIdentity>`
and obtain server-derived `user_id()` / `session_id()`. Keep public auth and
liveness routes outside this layer. Existing OpenAPI paths remain unchanged.

Clients send exactly one `Authorization: Bearer <access_token>` header. Only
the existing `ca1_` credential is accepted; query/body/cookie credentials and
caller owner IDs confer no identity. Expired access, revoked families, unknown
tokens, wrong-purpose secrets and malformed headers return 401 JSON
`code: invalid_access_token` with a Bearer challenge. Missing/unavailable
storage returns 503 `code: authentication_unavailable`. Every protected
response carries `Cache-Control: no-store`. Never log tokens; use TLS outside
local loopback development.

Lookup is read-only, uncached and does not extend session/token expiry. Pending
users can authenticate without financial authority. Revocation committed before
the lookup snapshot is enforced; an already admitted handler is not cancelled
by later revocation. Ownership and transactional financial permission remain
separate obligations. See [ADR-018](../architecture/decisions/018-authenticated-identity.md).

Run the transport cases with `cargo test -p cuenora-api --test auth_identity
--locked` from `backend/`. Run the three DB groups with synthetic disposable
PostgreSQL from the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_auth_identity_against_disposable_postgres -v
```

### Pending user registration (P04.02)

The local API accepts `POST /v1/auth/register` with only JSON `email` and `password`.
An accepted request returns `202` and `{"status":"accepted"}` for both a new and
an existing identity. Duplicates never replace credentials or profile data. No
session, financial access or verification email is created.

For a synthetic local database, apply all numbered migrations in
[`backend/migrations`](../../backend/migrations) in order before registration.
The API does not initialize or migrate a database. Export its connection URL
explicitly in the same shell before `cargo run`; the API does not consume
`DATABASE_URL` or load `.env`:

```bash
read -r -s -p "Synthetic local PostgreSQL URL: " CUENORA_DATABASE_URL
printf '\n'
export CUENORA_DATABASE_URL
cd backend
cargo run -p cuenora-api --locked
```

Enter a complete `postgres://cuenora:<percent-encoded-password>@127.0.0.1:<port>/cuenora`
URL using your local password and port; do not paste real credentials into shell
history, screenshots or reports. SQLx follows libpq defaults for omitted URL
fields, so supply explicit user/password/host/port/database. Missing configuration
keeps liveness operational and returns registration `503`; invalid configuration
stops startup with a credential-free error. Unset `CUENORA_DATABASE_URL` to return
to liveness-only database independence.

From another terminal, submit synthetic input:

```bash
curl --include http://127.0.0.1:3000/v1/auth/register \
  -H 'Content-Type: application/json' \
  --data '{"email":"synthetic+tag@example.test","password":"violet hills beside summer rivers"}'
```

| Boundary | Local behavior |
| --- | --- |
| Email/password | ASCII local part, IDNA domain, case-insensitive comparison; 15–128 Unicode scalar password values, spaces/Unicode permitted, common passwords rejected offline. Exact password bytes are hashed. |
| Invalid input | `400`, `code: invalid_request`; unknown/duplicate fields are rejected. |
| Body/content type | 8 KiB maximum JSON body (`413`); missing/unsupported JSON content type (`415`). Both use the same invalid-request body. |
| Unavailability | `503`, `code: registration_unavailable` for full admission, hashing failure, missing configuration or database failure. No database details or submitted values are returned. |
| Work limits | Two combined registration/login operations per app instance; hashing and verification run off the async executor. Cancellation retains admission until actual blocking work ends. Four pool connections; three-second acquire, five-second SQL statement and two-second lock timeouts. |

These are local resource bounds, not production rate limiting (P04.08).
Verification/recovery and production deployment remain unimplemented. A lost
connection around commit has an uncertain result; duplicate-safe retry is not an
exactly-once delivery guarantee. See [ADR-011](../architecture/decisions/011-bounded-registration-http.md)
and the [OpenAPI contract](../../contracts/openapi.yaml).

### Opaque login (P04.03)

`POST /v1/auth/login` accepts only JSON `email` and `password` within the same 8 KiB
body limit. Login checks the existing credential exactly, without applying
registration's minimum-length or common-password policy; inputs above 128
Unicode scalar values or 512 UTF-8 bytes are rejected. Unknown, credential-less
and incorrect-password cases share `401`, `code: invalid_credentials`.

A successful login creates a new session family and returns `access_token`,
`refresh_token`, `token_type: "Bearer"`, `expires_in: 900`, and
`refresh_expires_in: 2592000`, with `Cache-Control: no-store`. Tokens are opaque;
the server stores only their digests. The response sets no cookies and grants no
financial authority. Verification/storage/admission failures return the generic
`503`, `code: login_unavailable`. Bearer validation and token
rotation/revocation remain future work.

Run adapter and HTTP database checks against fresh disposable PostgreSQL:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_registration_store_against_disposable_postgres \
  PostgresComposeTest.test_registration_http_against_disposable_postgres \
  PostgresComposeTest.test_login_http_against_disposable_postgres -v
```

The existing Compose harness applies migrations and supplies random credentials
through environment variables, never arguments. It explicitly executes four
registration-store, four registration-HTTP and four login-HTTP cases ignored by normal Cargo tests; CI includes them in
the infrastructure suite. The helper constrains the destination to loopback;
no developer `.env` or personal application database is used. Do not manually
point ignored tests at a personal database. Unit and HTTP no-database tests run
in normal Cargo checks, including bounded admission and cancellation probes.

### Fresh migration verification (P03.11)

For the dedicated fresh-instance check:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_all_migrations_reproduce_schema_on_fresh_instances -v
```

This applies every SQL migration in filename order to two independent, disposable
PostgreSQL instances. It checks all nine application tables, validated constraints,
the final financial constraints, identical schema-only dumps and category seeds,
and empty non-catalog tables. Only the dump's random psql safety-token lines are
excluded from comparison. Existing cleanup checks verify removal of test
containers, volumes and networks. CI runs this test as part of the existing suite;
no new migration runner or application database connection is introduced.

**Verified locally on 2026-10-25:** the focused test passed (1 test, 19.848 s)
and the complete infrastructure suite passed (34 tests, 698.182 s), using the
local `postgres:18-bookworm` image. A disposable mutation probe that omitted
`0008_financial_constraints.sql` failed at the expected final-constraint assertion,
confirming that the new test detects an incomplete chain. Cleanup completed in
all three runs. This is a test-only addition over existing schema behavior, so
there is no production behavior change requiring a RED-to-GREEN implementation.

### SQL constraints and owned relationships (P03.12)

Run the focused same-currency ownership checks with:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_transactions_same_currency_cross_owner_inserts_and_updates \
  PostgresComposeTest.test_movements_same_currency_cross_owner_links_roll_back -v
```

Both users have valid SEK accounts and events. Positive controls prove that their
own relationships work; negative cases reject foreign account/event references
without relying on a currency mismatch. Transaction cases cover insertion, edits,
owner changes and both transfer sides. Movement cases isolate account ownership
from transaction ownership in both directions. Rejected requests must return a
nonzero exit status and SQLSTATE `23503`; existing snapshot helpers also verify
that preceding valid writes roll back and stored data is unchanged.

The full suite also retains the earlier constraint checks:

| Area | Existing acceptance coverage |
| --- | --- |
| Identity and sessions | Unique email keys/digests, credential storage, owner/session/successor references and rotation-state shape. |
| Accounts and categories | Supported types/currencies, required fields, owner-scoped references and read-only built-ins. |
| Transactions | Amount/date/state/transfer shape, category-kind references and owned account links. |
| Movements and audit | Required fields, exact bounds, owned event/account/actor links and append-only rejection. |
| Financial compatibility | Currency and savings-income restrictions, referenced account changes and atomic migration validation. |

These checks enforce database relationship integrity, not authenticated row-level
access. The API still has no authentication or financial database access; server
read/write authorization and session protocols remain Phase 04 responsibilities.
No SQL migration or access policy is changed by these additional tests.

**Verified locally on 2026-10-25:** both focused tests passed (2 tests, 37.216 s),
followed by the complete suite (36 tests, 735.272 s). An isolated mutation probe
removed the transaction account-owner and account-currency foreign keys and
confirmed that the new transaction test detects the now-accepted invalid write.
All disposable resources were cleaned. The committed migration files were never
changed. These tests exercise existing guards, not a new production behavior;
the mutation probe establishes that the added assertions detect relaxed guards.

### Existing schema behavior coverage

P03.09 adds four `test_audit_*` groups covering minimal audit events, owned actor/
transaction links, field-name-only summaries, append-only rejection and atomic DDL.
See [audit storage](../architecture/transaction-audit-storage.md).
P03.10 adds `test_financial_*` groups for currency/savings rules, compatible account
changes, existing-data validation and two-session FK concurrency; see
[financial constraints](../architecture/financial-constraints.md).

This suite also applies `backend/migrations/*.sql` to fresh disposable databases
and checks identity uniqueness, credential/session ownership, rotation-state
storage and transactional DDL failure. See [identity storage](../architecture/identity-storage.md)
for migration ordering, recovery boundaries and application responsibilities.
The API does not connect to these tables yet.
The same suite checks [account storage](../architecture/account-storage.md),
including supported fields, owner-scoped relationships and failed-DDL rollback.
It also checks [category storage](../architecture/category-storage.md): exact shared
built-ins, stable identifiers, read-only guards and transactional seed/DDL rollback.
It also checks [transaction storage](../architecture/transaction-storage.md): exact
event forms, lifecycle values, owned references and failed-write/DDL rollback.
[Account movement storage](../architecture/account-movement-storage.md) adds signed
postings, append-only guards, compensations, owned references and query indexes.

The [official image documentation](https://github.com/docker-library/docs/blob/master/postgres/README.md)
explains initialization and PostgreSQL 18's `/var/lib/postgresql` volume layout.
Keep the major version at 18; major upgrades and existing data need a separate
upgrade plan. Pulling `postgres:18-bookworm` can update its minor release.

## Continuous integration

The workflow `.github/workflows/ci.yml` runs for every pull request, on pushes
to `main`, through manual dispatch, and weekly on Monday at 07:17 UTC. It uses a
read-only GitHub token and has three jobs:

| Check name | Current behavior |
| --- | --- |
| `Repository / Validate` | Checks required files, UTF-8, LF newlines, final newline, trailing whitespace, and merge markers. |
| `Backend / Rust` | Runs format, Clippy, Rust tests and isolated PostgreSQL migration/infrastructure tests. |
| `Backend / Audit` | Audits the committed Cargo.lock against fresh RustSec advisories and registry yanks. |

`Backend / Rust` runs:

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features --locked
```

`backend/Cargo.lock` is tracked so that `--locked` enforces reproducible
dependency resolution.

The same job then runs `python3 tests/infrastructure/test_postgres_compose.py`
from the repository root, using Docker Compose and disposable local databases.
No database credentials or external mail service are needed in CI.

### Dependency audit

From `backend/`, install the pinned development tool and audit the lockfile:

```bash
cargo install cargo-audit --version 0.22.2 --locked --no-default-features
cargo audit --file Cargo.lock --deny warnings
```

See the [dependency audit policy](DEPENDENCIES.md) for blocking findings, network
requirements, and exception handling. No advisory ignores are configured.

### Required checks

After the first pull request runs CI successfully, configure the branch ruleset
for `main` to require **`Repository / Validate`**. Require
`Backend / Rust` once its real workspace checks pass in the pull request.
Branch ruleset changes are a separate maintainer action.

The workflow intentionally uses no path filters: required checks should run on
every pull request, even when only documentation changes.

## Dependency updates

`.github/dependabot.yml` enables weekly update pull requests for GitHub Actions.
Cargo dependencies are audited by `Backend / Audit`; automated Cargo updates
are not configured yet. Gradle updates remain deferred until that project exists.

## Working conventions

- Use feature branches and pull requests. Do not push directly to `main`.
- Keep documentation, commits, and pull request descriptions in English.
- Never commit `.env` files, credentials, signing keys, or personal financial data.
- Run the repository checks locally before opening a pull request.
- Configure Android and iOS pipelines only when their corresponding projects exist.

## Next milestone

The workspace, PostgreSQL harness, authentication adapters and money/account
domain foundations exist. Continue from the [current implementation order](../plans/master-plan.md#13-current-implementation-order),
not the completed scaffold tasks. P05.02 introduces the account API slice with
its verification/ownership prerequisites; dated postings also need confirmed-zone
validation. Financial workflows, email verification/recovery, database readiness
and mobile projects remain pending. The [documentation map](../README.md) links
approved policies to implementation evidence.

## Authentication abuse protection (P04.08)

A single app instance shares bounded in-memory fixed 60-second windows:
registration **5/client IP**, login **20/client IP** and **10/canonical email**
across IPs, refresh **60/client IP**. These are initial defaults, not a capacity
guarantee. IPv4-mapped addresses normalize to IPv4; IPv6 clients share a /64.
Every attempt counts, even malformed or successful requests. Email comparison
reuses the registration policy (IDNA and case-insensitive; no dot/plus removal).
An exhausted budget returns typed `429 rate_limited`, positive integer
`Retry-After` seconds, `Cache-Control: no-store` and the usual request ID.
Blocked attempts do not extend the window. IP protection runs before body
validation; login-identifier protection runs before password or database work.

At most 8,192 key slots exist; active entries are never evicted to make room for
new clients. Missing/untrusted client identity, a full store or a poisoned lock
returns typed `503 protection_unavailable`, without performing authentication.
The 8 KiB payload and existing password/refresh concurrency safeguards remain.
Health, unmatched paths and unsupported methods retain their previous behavior.

### Direct local development

Leave `CUENORA_TRUSTED_PROXY_IPS` unset. The binary attaches the TCP peer address;
all forwarded IP headers are ignored. In-process router callers must attach
`axum::extract::ConnectInfo<SocketAddr>` (or serve using
`into_make_service_with_connect_info::<SocketAddr>()`); missing metadata fails
closed on sensitive POST routes. Tests may supply synthetic transport metadata,
but a public HTTP header must never supply the immediate peer.

### Cloudflare proxied DNS → Dokploy/Traefik → API

The maintainer's intended deployment uses orange-cloud DNS and one API instance,
not Cloudflare Tunnel. **The following is a deployment checklist, not verified
server configuration. Do not expose the API with these prerequisites unmet.**

1. Restrict origin HTTP(S) ingress to the maintained
   [Cloudflare address ranges](https://developers.cloudflare.com/fundamentals/concepts/cloudflare-ip-addresses/),
   covering both IPv4 and IPv6. Orange-cloud DNS alone does not close the origin.
2. Isolate API ingress so only the intended Traefik proxy can reach it. Do not
   publish an API container port or trust all Docker/private networks. The current
   binary still binds `127.0.0.1:3000`; container listening/networking setup is a
   separate deployment prerequisite, not implemented by this rate-limit change.
3. Configure `CUENORA_TRUSTED_PROXY_IPS` with the **exact immediate TCP peer IPs**
   the API observes from Traefik. For example, the synthetic value
   `192.0.2.10,2001:db8::10` illustrates syntax, not usable deployment addresses.
   At most 32 IPv4/IPv6 addresses; no CIDRs or wildcards. Empty, non-Unicode or
   malformed settings stop startup with a fixed safe error. Keep proxy addresses
   stable and update this setting when networking changes.
4. Ensure Traefik preserves exactly one valid `CF-Connecting-IP`. The API uses it
   only when the immediate peer is configured as trusted; missing, malformed or
   repeated values fail closed. `X-Forwarded-For` and `X-Real-IP` are never client
   identity fallbacks. Traefik's
   [`forwardedHeaders.trustedIPs`](https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/)
   concerns `X-Forwarded-*`; it does not authenticate arbitrary Cloudflare headers
   or replace origin ingress restrictions. Do not set `forwardedHeaders.insecure`.
5. Validate the actual chain with synthetic traffic before exposure: two clients
   must receive independent budgets; changing forwarded headers from a direct
   client must not create a new budget; bypassing Cloudflare must be impossible.
   This guide assumes no Cloudflare Worker/header transform changes and no
   Pseudo IPv4 overwrite mode. Check
   [Cloudflare header semantics](https://developers.cloudflare.com/fundamentals/reference/http-headers/)
   before changing those settings. Use TLS for public traffic and origin traffic.

Counters reset on process replacement and do not coordinate replicas. Fixed
windows permit boundary bursts; NAT/shared IPv6 networks share budgets. The
identifier budget also applies to unknown emails and never permanently locks an
account, but an attacker can temporarily exhaust a known identifier. These
controls do not replace Cloudflare/Traefik edge protection, recovery design or
monitoring. Never log raw email, IP, passwords or tokens to diagnose throttling.
Revisit shared storage before adding a second API instance. See [ADR-020](../architecture/decisions/020-authentication-throttling.md).

### Local verification

```bash
cd backend
cargo test -p cuenora-api --test auth_throttling --locked
cargo test -p cuenora-api auth_throttling::tests --locked
```

The existing disposable PostgreSQL infrastructure harness additionally runs the
ignored throttle test: rejected valid registration, login, and refresh writes
must preserve the full user/credential/session/access/refresh snapshot. Run only
through `python3 tests/infrastructure/test_postgres_compose.py` from the root;
never supply a personal or production database.


## Logout and global session revocation (P04.11)

See [ADR-021](../architecture/decisions/021-session-logout.md) and the
[OpenAPI contract](../../contracts/openapi.yaml).

| Endpoint | Credentials and outcome |
| --- | --- |
| `POST /v1/auth/logout` | JSON `{ "refresh_token": "cr1_<opaque>" }`; revoke only its family, including via expired/consumed generations. Empty 204 also for unknown/malformed secrets and already revoked families. |
| `POST /v1/auth/logout-all` | `Authorization: Bearer ca1_<opaque>` plus JSON `{ "password": "<current password>" }`; empty 204 after revoking every session of that user, including the caller. |

Both require strict JSON up to 8192 bytes; unknown/duplicate fields are rejected.
Global logout uses the existing exact-byte password limits (128 Unicode scalars,
512 UTF-8 bytes), bounded Argon2id verification and server-derived owner/session.
Wrong/stale password returns generic 401 `invalid_credentials`; invalid bearer
uses the existing challenge. Storage/hash/admission failures return 503
`authentication_unavailable`; payload errors use 400/413/415 `invalid_request`.
All responses follow request-ID/no-store policy, with no cookies or new tokens.
Clear locally held secrets after logout; a lost commit response may be ambiguous.
A global logout retry after success needs a new login.

Global revocation shares login's credential lock and then locks families in UUID
order. A login ordered before it is included; one after commit may open a new
session. Refresh is serialized on family locks. In-flight handlers already
admitted by middleware are not cancelled. Token history and prior revocation
instants remain unchanged; no migrations or new dependencies are required.

Attempt budgets per 60-second process-local window: logout 60/client IP,
logout-all 20/client IP plus 10/authenticated user across IPs. These are separate
keys, with the same explicit proxy trust and bounded store as P04.08. Global
logout shares registration/login hashing admission; single logout shares refresh
DB admission. Limits can temporarily reject legitimate clients under contention.

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_logout_http_against_disposable_postgres -v
```

The harness requires seven passing DB groups, including both login/global lock
orders, blocked refresh, scoped/idempotent logout, password reauthentication,
per-user attempt limits, stale credentials/caller rejection, refresh-before-global,
cancellation rollback and atomic storage failure. Direct Cargo tests run only
transport coverage and intentionally ignore these DB groups.

## Uniform versioning and account creation (P05.02)

All implemented routes use `/v1`, including `/v1/health/live` and `/v1/auth/*`;
old paths return 404. Apply all migrations through `0010_account_creation_keys.sql`
before using `POST /v1/accounts`. Follow the [account creation contract](../product/account-creation.md)
and [ADR-023](../architecture/decisions/023-versioned-account-creation.md).
No verification bypass exists; the email delivery/confirmation runtime remains
pending, so registration → verification → account creation is not yet usable.

The disposable harness includes seven ignored `account_http` PostgreSQL cases.
The ordinary Cargo suite checks bearer failure without storage and route versioning;
it does not run ignored database acceptance tests. Run the focused DB target with
`python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_account_http_against_disposable_postgres`.

## Dated opening balances and profile zones (P05.03)

Local implementation passed own/independent checks and native review; publication is pending. Apply migrations through
`0011_opening_creation_keys.sql` in order; existing P05.02 keys retain their result
and gain zero/no-date opening equivalence. Confirm an explicit IANA zone through
`PATCH /v1/me` before sending a dated opening to `POST /v1/accounts`; follow the
[opening contract](../product/opening-balances.md). Missing email verification
remains a hard financial gate; no delivery/confirmation bypass is provided.

New dependencies are Chrono for dates/captured UTC and chrono-tz for a compiled
IANA catalog, not the host/device time zone. Run the pure cases with
`cargo test -p cuenora-domain --test business_date --locked` from `backend/`.
Run isolated profile/opening cases from the root with:

```bash
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_profile_zone_against_disposable_postgres PostgresComposeTest.test_opening_balance_against_disposable_postgres PostgresComposeTest.test_opening_key_migration_preserves_existing_creation_results
```

The full existing infrastructure command also runs these targets. Ordinary Cargo
tests keep the PostgreSQL cases ignored; that is not database acceptance proof.
