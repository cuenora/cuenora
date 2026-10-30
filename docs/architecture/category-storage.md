# Global built-in category storage

P03.06 adds `backend/migrations/0004_categories.sql`: one shared catalog with
three income and seven expense categories. That migration creates no category API or custom
categories; P05.07 adds the reference read API described below. Existing identity and account storage is unchanged.

| Code | Kind | English label | UUID suffix |
| --- | --- | --- | --- |
| `salary` | `income` | Salary | `001` |
| `interest` | `income` | Interest | `002` |
| `other_income` | `income` | Other income | `003` |
| `housing` | `expense` | Housing | `004` |
| `groceries` | `expense` | Groceries | `005` |
| `transport` | `expense` | Transport | `006` |
| `utilities` | `expense` | Utilities | `007` |
| `health` | `expense` | Health | `008` |
| `leisure` | `expense` | Leisure | `009` |
| `other_expense` | `expense` | Other expense | `010` |

IDs are fixed UUIDs `10000000-0000-4000-8000-000000000NNN`, not generated on
signup. Codes are stable, unique machine keys, separate from display labels.
There is no `user_id`: every user sees the same catalog, with no provisioning
copies. Category membership does not authorize access to a user's transactions.
The `(id, kind)` unique target allows later foreign keys to reject an expense
using an income category. No production child table is introduced here.

## Read-only boundary

When reached, a statement trigger installed after seeding rejects ordinary
`INSERT`, `UPDATE`, `DELETE` and `TRUNCATE`, including zero-row statements, with
SQLSTATE `55000`. Once another table references categories, PostgreSQL can reject
a standalone `TRUNCATE categories` earlier with `0A000` (foreign-key preflight).
Both rejections roll back the enclosing transaction, including preceding writes
in that transaction. This freezes the catalog, not just its labels. There are no
runtime audit fields: catalog revisions belong to version-controlled migrations,
not user financial events.

The guard is not authentication or protection against an administrator who can
disable triggers or alter the schema. Future application roles must use least
privilege; category APIs must not expose built-in mutations. Future catalog
maintenance requires a controlled migration that intentionally replaces the
guard; customization needs separately scoped user-owned storage. Neither is
implemented in this slice. The [PostgreSQL trigger contract](https://www.postgresql.org/docs/18/sql-createtrigger.html)
describes statement events, including `TRUNCATE`.

## Apply and verify

Use the [existing transactional migration procedure](identity-storage.md#apply-verify-and-recover).
Run `python3 tests/infrastructure/test_postgres_compose.py` from the root with
local Docker Compose available. Tests check exact catalog reproduction, shared
reads by non-owner roles, mutation rollback even with explicit write grants,
kind references and failed-DDL rollback. Reapply fails
without changing stored data. A late function-name collision rolls back the new
table and seed while preserving prior identity/account data and the conflicting
function. Do not drop populated databases to deploy corrections.

## Read-only HTTP catalog (P05.07)

`GET /v1/categories` requires a valid bearer session, but not email verification
or a confirmed profile zone. It returns `{"categories": [...]}` with all ten
built-ins in ascending UUID order. Each entry contains `id`, `code`, `kind`, and
`name` exactly as stored. Both pending and verified users see the same catalog;
this grants no access to personal financial records. Responses use `no-store`.
No query parameters or body are accepted (400; oversized bodies return 413).
HEAD uses GET semantics without a body; POST/PUT/PATCH/DELETE are unsupported.
There are no custom-category, filtering or pagination operations.

Missing, invalid, expired or revoked credentials return 401. Storage failures
return a safe 503 `category_listing_unavailable`; identity lookup failures retain
`authentication_unavailable`. Reads use bounded SQL timeouts and never write.
See the [OpenAPI contract](../../contracts/openapi.yaml).

Category-kind compatibility already uses `transactions.category_kind` and the
composite foreign key to `(categories.id, categories.kind)`: ordinary income and
credited interest accept income categories, expenses accept expense categories,
and transfers/openings cannot carry a category. The isolated storage tests cover
rejected cross-kind references without persisted changes. Transaction HTTP
creation/editing remains P05.08 onward; no unused validation API is added here.

Run the HTTP and existing compatibility acceptance checks with:

```bash
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_categories_against_disposable_postgres PostgresComposeTest.test_transactions_owner_and_category_relationships
```
