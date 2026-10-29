# Account storage and ownership

P03.05 adds `backend/migrations/0003_accounts.sql`. It stores the ownership root
for later financial records, with isolated PostgreSQL acceptance tests. Account
HTTP actions and financial postings remain later work.

| Field | Storage contract |
| --- | --- |
| `id` | PostgreSQL-generated UUID primary key. |
| `user_id` | Required existing user; restrictive deletion and an owner lookup index. |
| `name` | 1–255 characters; rejects ASCII control characters and ASCII-whitespace-only names. |
| `account_type` | Exactly `normal` or `savings`. |
| `currency` | Exactly `SEK`, `EUR` or `USD`; text guards reject lowercase and padding. |
| `archived_at` | NULL for active storage state; an instant represents archived state. |
| `created_at`, `updated_at` | Microsecond instants with statement-time defaults; updates must maintain `updated_at` explicitly. |

Audit/archive instants use the identity schema's years 0001–9999 bounds, excluding
infinity. Applications must serialize instants in UTC. The name guard is bounded
storage validation, not a complete Unicode normalization or UI policy. Names are
not unique in this schema. P05.01 defines the [domain name policy](../product/accounts.md)
with outer-whitespace trimming; API and persistence integration remain pending.

The unique `(id, user_id)` constraint is a target for later composite foreign
keys. Tests use ordinary child tables in disposable databases to prove valid owner relationships work
and cross-owner relationships fail atomically. This does not authenticate a
request or restrict arbitrary SQL reads by user. Server ownership checks and the
verified-email/profile-zone gates remain application responsibilities.

No balance, opening amount or interest terms live in `accounts`. Posted account
movements will determine balances; opening postings are a separate business
operation. P03.10 now constrains type/currency changes to preserve existing
references; see [financial constraints](financial-constraints.md). Owner changes,
rename/archive/restoration protocols and account deletion policy remain separate.
The original account migration adds no
mutability trigger, row-level security policy or production child table.

Apply and recover through the [existing migration procedure](identity-storage.md#apply-verify-and-recover).
Files contain forward DDL without transaction control; the current disposable
runner applies each file with `--single-transaction` and `ON_ERROR_STOP`. Reapply
fails unchanged. A failed index creation rolls back the newly created account
table while preserving prior identity tables and existing data. Once populated,
deployment corrections need a controlled forward/backup plan rather than dropping
account data.

From the repository root, run `python3 tests/infrastructure/test_postgres_compose.py`.
The existing CI entry point also runs the account tests. SQLx and account API
connections have not been introduced by this slice.

## P05.02 creation integration

The original storage-only scope above is historical. Creation now uses SQLx and
the domain name/type/currency policy through `POST /v1/accounts`; own and independent checks
and native review passed; not yet published. Migration `0010_account_creation_keys.sql` adds atomic lifetime
idempotency with owner-bound account references and the original result. See the
[creation contract](../product/account-creation.md) and [ADR-023](decisions/023-versioned-account-creation.md).
Opening postings, account queries and archive behavior remain separate work.
