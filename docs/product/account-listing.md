# List accounts

**Roadmap:** P05.04–P05.05 · **Status:** Metadata listing merged; balances verified locally with native review acknowledged, unpublished

`GET /v1/accounts` returns one bounded page of the authenticated owner's account
metadata and current balances. It includes active and archived accounts; the
[management workflow](account-management.md) does not hide or delete history.
Opening amounts, movements and audit data are not exposed by this listing.

## Request and response

Send an active bearer token. The user's email must currently be verified; a
confirmed profile time zone is not required. Ownership comes only from the
bearer identity. Query parameters are strict and unique; unknown fields and
duplicate parameters return `400 invalid_request`.

| Parameter | Contract |
| --- | --- |
| `limit` | Optional integer from 1 through 100; defaults to 50. |
| `cursor` | Optional opaque cursor returned by the previous page. It must identify an account owned by the caller. |

Pages sort by `created_at` ascending, then account ID ascending. The cursor is
the ID of the last returned account only when another page exists; otherwise
`next_cursor` is `null`.

```json
{
  "accounts": [
    {
      "id": "00000000-0000-0000-0000-000000000001",
      "name": "Daily",
      "account_type": "normal",
      "currency": "SEK",
      "status": "active",
      "balance": "123.45"
    }
  ],
  "next_cursor": null
}
```

Each account has exactly six fields: `id`, `name`, `account_type`, `currency`,
`status`, and `balance`. Names are returned as stored, including legacy whitespace, and
duplicate names remain distinct accounts. `status` is `active` when `archived_at`
is null and `archived` otherwise.

GET bodies must be empty; nonempty bodies are rejected, and the request body is
bounded to 8192 bytes. Query strings are bounded to 1024 bytes. The listing uses
a short repeatable-read, read-only transaction so verified status, cursor
ownership and the selected page share one database snapshot. Reads never write
or normalize accounts.

## Current balances (P05.05)

`balance` is a decimal string with exactly two fractional digits, in the account's
existing `currency`. Sum every append-only signed posting, including opening and
compensating movements. Do not filter postings using the current transaction
state: voiding an event does not remove its original posting. An account without
movements returns `"0.00"`; negative balances are valid. Accountless events and
other accounts/currencies never enter this sum.

Balances share the verification/cursor/page snapshot. A subsequent page or
request can observe newly committed movements; traversal is not a historical
snapshot. No as-of/date filter, cross-account total or client balance is accepted.
The exact final sum must fit `-92233720368547758.07` through
`92233720368547758.07`; otherwise the entire page fails with the existing safe
503, without rounding, saturation, partial results or writes. An out-of-range
account on a later page does not fail the current page. POST creation responses
remain the original five metadata fields, with no balance. See
[ADR-026](../architecture/decisions/026-exact-account-balances.md).

## Errors and verification

| Failure | Status / code |
| --- | --- |
| Missing, invalid, expired or revoked bearer | 401 `invalid_access_token` |
| Email is not currently verified | 403 `email_not_verified` |
| Invalid limit, query field, cursor or nonempty GET body | 400 `invalid_request` |
| GET body exceeds 8192 bytes | 413 `invalid_request` |
| Database lookup, timeout, transaction failure or unrepresentable balance | 503 `account_listing_unavailable` |

Unknown and foreign-owner cursors have the same generic response. All responses
use the existing request-ID and `Cache-Control: no-store` policy. A storage error
is never represented as an empty successful page.

Run the ordinary transport checks and isolated PostgreSQL acceptance cases from
the repository's [development setup](../development/SETUP.md). Listing does not
change `POST /v1/accounts` creation behavior.

The accepted snapshot/cursor tradeoffs are recorded in [ADR-025](../architecture/decisions/025-owner-scoped-account-listing.md).
