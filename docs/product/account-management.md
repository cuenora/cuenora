# Rename, archive and reactivate accounts

**Roadmap:** P05.06 · **Status:** Implemented locally; own/independent proof passed; native review acknowledged, unpublished

`PATCH /v1/accounts/{id}` updates an authenticated, email-verified owner's account
metadata. There is no deletion endpoint. Account IDs, owner, type and currency
remain unchanged, as do financial events, append-only movements, audit and
original account-creation idempotency records.

## Request and response

Send an active bearer and `Content-Type: application/json`. At least one field
is required; all fields are optional individually but explicit null is invalid.
Unknown/duplicate fields and all query parameters are rejected. JSON is bounded
to 8192 bytes. The ID uses canonical UUID syntax (hex is case-insensitive).

```json
{"name":"Daily expenses","status":"archived"}
```

| Field | Policy |
| --- | --- |
| `name` | Reuse the [account name policy](accounts.md#name-policy): trim outer Unicode whitespace, 1–255 Unicode scalar values, no remaining ASCII controls, preserve case/internal whitespace/Unicode, duplicates allowed. Active and archived accounts may be renamed. |
| `status` | `active` or `archived`. No deletion or other lifecycle state. |

A successful request returns `200` with five current metadata fields:

```json
{
  "id":"00000000-0000-0000-0000-000000000001",
  "name":"Daily expenses",
  "account_type":"normal",
  "currency":"SEK",
  "status":"archived"
}
```

A profile time zone is not required for undated metadata updates. A PATCH does
not accept a balance, currency, type or owner. Read current balances through
[GET /v1/accounts](account-listing.md), which retains archived accounts.

## Archive and reactivate

An active account may be archived only if its exact persisted signed posting
sum is zero. Count opening and compensating postings regardless of current event
state. No postings means zero. A nonzero total, including a total beyond the
Money output range, rejects the entire request with `409 account_balance_not_zero`;
a combined rename is not partially applied. No balance is accepted from a client.

Archiving preserves queryable history and balance. Archived accounts must reject
new income, expenses and transfers when those posting endpoints are introduced.
Reactivation enables such new operations again without modifying historical
records. P05.13 [corrections](transaction-corrections.md) require reactivation before
movement-affecting edits/voids on any affected archived account. Description and
category corrections remain allowed without changing account movements.

Repeat assignments are no-ops: unchanged name/state does not rewrite
`updated_at` or `archived_at`. Repeating `status: archived` for an already archived
account does not start another archive transition or revalidate its historic
balance. Reactivating a legacy archived account does not erase its balance.

## Atomicity, concurrency and retry

Lock the authenticated owner, then the owned account, in one READ COMMITTED
transaction. Recheck verified email and ownership; after any lock wait, read the
exact posting sum in a new statement before archiving. Future financial posting
use cases must use the same owner→account lock order, check active state while
holding those locks, then post atomically. Composite foreign keys are not a
substitute for that runtime authorization. No new posting route exists here.

Concurrent updates serialize. Omitted fields retain the latest locked values;
updates of the same field are last-writer-wins, without ETags or optimistic
versioning. PATCH uses idempotent state assignments, not stored lifetime
`Idempotency-Key` replay. A lost response/commit acknowledgement can be retried,
but an intervening edit can change what reapplying that assignment means.

Creation POST replays still return the original creation response (including its
original name and active status), not the current mutable metadata, and never
reactivate or repost an archived account. Use GET to read current state.

## Errors

| Failure | Status / code |
| --- | --- |
| Missing, invalid, expired or revoked bearer | 401 `invalid_access_token` |
| Email not currently verified | 403 `email_not_verified` |
| Unknown or foreign-owned account | 404 `not_found`, identical generic response |
| Malformed UUID, JSON, fields, name, status or query | 400 `invalid_request` |
| JSON exceeds 8192 bytes / unsupported media | 413 / 415 `invalid_request` |
| Active→archived with nonzero balance | 409 `account_balance_not_zero` |
| Database/lock/transaction failure | 503 `account_update_unavailable` |

All responses use request IDs and `Cache-Control: no-store`. Rejected requests
leave persisted state unchanged. Safe unavailable errors disclose no account
name, balance or database diagnostics and never claim a false success.
