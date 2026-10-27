# ADR-011: Bounded pending-registration HTTP

- **Status:** Accepted; implemented and independently verified
- **Date:** 2026-10-26
- **Scope:** P04.02 HTTP boundary; ADR-009 policy and ADR-010 persistence unchanged

P04.07 [ADR-019](019-typed-api-errors.md) supersedes only the legacy error-body
shape below; status codes, stable error codes and security headers are unchanged.

## Decision

Expose local `POST /auth/register` through the API library router and binary.
Accept only JSON email/password; reject unknown/duplicate fields and map extractor
errors to credential-free categories. Limit the JSON body to 8 KiB, accommodating
valid escaped Unicode input. Reuse the existing policy and atomic store; no new
migration or parallel normalization/identity path is introduced.

Return identical `202 {"status":"accepted"}` for valid new and duplicate requests.
Hash before attempting persistence in both cases; never preflight account
existence or replace an existing credential. This response provides no user ID,
session, financial authority or promise of verification email delivery. Validation
uses `400`, size `413`, content type `415`, and unavailable work/storage `503` as
specified in OpenAPI. No submitted values or database errors are logged/returned.

Two non-waiting semaphore permits per router instance bound admitted registration
work. Move the permit into `spawn_blocking`, then return it with the result and
hold it through persistence. Cancelling the HTTP future cannot release the permit
while the non-cancellable blocking hash still runs. This avoids an unbounded
blocking-task queue without moving CPU work onto the async executor.

The binary reads only explicit `CUENORA_DATABASE_URL`, without `.env` loading or
implicit `DATABASE_URL`. A missing setting disables registration persistence but
preserves exact process liveness. A syntactically invalid setting fails startup
with a safe fixed message. A lazy SQLx pool keeps availability independent of
liveness: maximum four connections, three-second acquisition, five-second server
statement and two-second lock timeouts. Disable statement logging. SQLx/libpq
fallbacks for omitted URL fields still apply; development instructions require a
complete explicit local URL. Binding remains `127.0.0.1:3000`.

## Tradeoffs and limits

Fixed local bounds are deliberately smaller than Tokio's default blocking-thread
capacity. They reject excess requests rather than queueing costly work. They are
not per-user/IP rate limiting, a total connection limit or production readiness.
They do not implement P04.08, login, email verification, recovery or financial
routes. SHA/Argon benchmark timings are not a capacity guarantee.

Database cancellation and lost responses may leave an uncertain commit result;
the single-statement store and duplicate-safe retry preserve identity invariants,
not exactly-once network delivery. Existing schema constraints remain authoritative.

## Verification

Public-router tests cover strict JSON/policy rejection, safe no-database errors,
unchanged liveness, persisted pending identity and actual hash verification,
case/IDNA duplicates, bounds/Unicode/space/NUL preservation, distinct dot/plus
identities, equivalent concurrent requests, trigger-forced rollback and closed
pool failure. Deterministic unit probes cover full admission and cancellation of
the awaiting future while blocking work remains active. Compose explicitly runs
ignored database cases against fresh loopback-only PostgreSQL, including in CI.
No claim about production readiness or completed Phase 04 follows from these tests.

Sources: [Axum JSON body limits](https://docs.rs/axum/0.8.9/axum/extract/struct.DefaultBodyLimit.html),
[Tokio blocking work and cancellation](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html),
and [SQLx PostgreSQL connection options](https://docs.rs/sqlx/0.8.6/sqlx/postgres/struct.PgConnectOptions.html).

On 2026-10-26 own checks passed: 31 normal Cargo tests, eight explicitly run
database cases, fmt and Clippy with warnings denied. The full infrastructure
suite passed 38 tests (757.663 s) before supplemental test-only assertions; the
strengthened focused HTTP cases and normal suite passed afterward. Independent
real-binary probes found no blockers, including production lock timeout, two-slot
fail-fast admission and exact baseline liveness comparison. The dependency audit
scanned 191 packages against 1,378 advisories with no vulnerabilities/yanks/warnings.
