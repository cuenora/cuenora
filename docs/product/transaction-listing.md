# Recorded transaction history

P05.12 adds `GET /v1/transactions` for the authenticated, currently verified
owner's recorded financial events. A confirmed profile zone is not required for
reading stored business dates. No request can supply a different owner.

## Query

| Parameter | Meaning |
| --- | --- |
| `limit` | Optional page size, default 50, range 1–100. |
| `cursor` | Opaque last-returned event UUID; copy `next_cursor` unchanged. Must identify an owned event matching the current filters. |
| `date_from` | Inclusive earliest `occurred_on`, strict `YYYY-MM-DD`, years 0001–9999. |
| `date_to` | Inclusive latest `occurred_on`, same format; cannot precede `date_from`. |
| `category_id` | Existing global category UUID. |
| `account_id` | Owned account UUID, or literal `none` for accountless events. A transfer matches either account, once. |

All supplied filters combine with AND. Omitted filters include every matching
event, including accountless records and archived-account history. Ranges may
extend into the future: a read does not create or revalidate business dates.
Dates are never converted through an audit timestamp or the current profile zone.

Pages sort by business date descending, then UUID descending, not creation time.
The cursor is an exclusive boundary; `next_cursor` is null when no further row
exists. Keep filters unchanged while traversing. Each request uses one read-only
repeatable-read snapshot, but pages across requests are not a frozen export:
edits, voids and backdated inserts between pages can change later results.

```text
GET /v1/transactions?date_from=2024-02-01&date_to=2024-02-29&account_id=none&limit=50
```

## Response

`transactions` is an array and `next_cursor` is a UUID string or null.
Every item has the following fields; nullable fields remain explicitly null:

| Fields | Representation |
| --- | --- |
| `id`, `kind`, `state` | Event UUID; `income`, `expense`, `transfer`, `opening_balance` or `interest_income`; `posted` or `voided`. |
| `amount`, `currency` | Exact decimal string with two places and SEK/EUR/USD. Signed openings remain signed; ordinary event amounts remain positive. |
| `occurred_on` | Stored canonical business date. |
| `account_id`, `category_id`, `description` | Stored reference/text or null. |
| `destination_account_id`, `received_amount`, `received_currency` | Transfer destination and exact received amount/currency; null for other kinds. |

Transfers are one event, not two list items. Both currencies and amounts remain
separate. Opening balances and transfers are not income/spending; credited
interest is distinct from forecast interest. Planned recurrences/forecasts are
not stored transactions and never appear. Voided events remain visible with their
state; this history is not a balance/report calculation. No totals are returned.

## Rejections and safety

A valid bearer is required (401 `invalid_access_token` otherwise). Unverified
owners receive 403 `email_not_verified`. Unknown/duplicate query keys, malformed
values, reversed dates, nonempty GET bodies, missing/foreign account references,
unknown categories, and missing/foreign/nonmatching cursors return generic 400
`invalid_request`. Unknown and foreign IDs are indistinguishable. Queries are
bounded to 1024 raw URI bytes; GET bodies to 8192 bytes (413 when exceeded).

Database failures return 503 `transaction_listing_unavailable`, never a partial
page. Standard authentication/request-ID errors remain possible. Responses are
`Cache-Control: no-store`, with the standard diagnostic request-ID envelope for
errors. Reads write no events, movements, audit records, balances or retry keys.

POST /v1/transactions retains its existing strict creation contract, including
rejecting nonempty query strings; GET parameters never alter creation behavior.
See [OpenAPI](../../contracts/openapi.yaml) and the
[execution record](../plans/completed/2026-10-30-transaction-listing.md).
