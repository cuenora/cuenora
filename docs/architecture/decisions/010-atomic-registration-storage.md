# ADR-010: Atomic pending-registration persistence

- **Status:** Accepted; adapter implemented, HTTP integration in ADR-011
- **Date:** 2026-10-26
- **Scope:** P04.02 persistence slice, after ADR-009 credential policy

## Decision

Use SQLx 0.8.6 with PostgreSQL, Tokio and Rustls, with default features disabled.
Runtime parameterized queries avoid a build-time database and do not introduce
a second migration runner. The existing schema remains unchanged.

Persist a user and its encoded credential through one data-modifying CTE. The
user insert returns its server-generated UUID to the credential insert. A
credential constraint failure aborts the entire statement, so no orphan user
survives. Verification instant and profile zone remain NULL by schema defaults.
No financial record or session is created.

The generated unique email key is the identity authority. `ON CONFLICT
(email_key) DO NOTHING` handles duplicates, including concurrent requests, without
preflight existence checks, credential replacement or profile changes. Return the
same `Ok(())` for a new registration and a duplicate. Do not expose affected-row
counts, user IDs, email existence or database error details through registration.

## Tradeoffs and boundary

One statement keeps atomicity and generated identity entirely in PostgreSQL; an
explicit multi-query application transaction would work but is unnecessary here.
The adapter cannot repair an existing identity missing a credential: duplicate
registration is not password reset, account linking or recovery.

Callers must provide an `Email` validated by ADR-009 and an encoded hash generated
by its hashing function, not client-supplied identity, verification state or PHC.
The adapter receives a bounded connection pool; admission, connection settings,
query timeouts, safe HTTP responses and off-executor hashing belong to the next
slice. A lost connection around commit can make a result uncertain; retrying a
registration is safe, but this is not an exactly-once delivery claim.

## Verification

Four Rust adapter tests explicitly run through the existing disposable Compose
suite, including in CI. They check pending identity/credential persistence,
case/IDNA duplicates with unchanged snapshots, same-key races, dot/plus separation,
credential-rejection rollback and unavailable-pool behavior. Normal Cargo tests
ignore these database cases; the infrastructure suite must execute them rather
than relying on that skip.

On 2026-10-26 the full infrastructure suite passed (37 tests, 751.422 s), as did
independent adapter probes and the 190-dependency audit with warnings denied.
Probes for 16 concurrent equivalent requests and a trigger-forced credential
failure with a valid hash are retained in the regressions; the strengthened
focused suite passed afterward (four database cases, 12.543 s).

Run from the repository root:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_registration_store_against_disposable_postgres -v
```

This adapter slice alone does not provide HTTP or startup configuration.
[ADR-011](011-bounded-registration-http.md) records the subsequent HTTP integration.

Sources: [SQLx 0.8.6](https://docs.rs/sqlx/0.8.6/sqlx/) and
[PostgreSQL data-modifying CTEs](https://www.postgresql.org/docs/18/queries-with.html#QUERIES-WITH-MODIFYING).
