# Recurrence listing API

The read endpoints expose a verified owner's recurrence metadata without
changing financial state. Planning rows remain distinct from posted expenses;
only the existing explicit confirmation operation creates a transaction.

## Endpoints

| Endpoint | Filter and order | Page fields |
| --- | --- | --- |
| `GET /v1/recurring-rules` | Optional `state=active\|paused`; UUID ascending. | `rules`, nullable UUID `next_cursor`. |
| `GET /v1/recurring-occurrences` | Optional `state=planned\|confirmed\|skipped` and `rule_id`; `due_on`, then UUID ascending. | `occurrences`, nullable opaque `next_cursor`. |

Both endpoints require an authenticated, email-verified owner. Pages default to
50 and allow 1–100 rows. They have no date-window restriction. Query fields are
strict and unique, raw queries are limited to 1024 bytes, and GET bodies must be
empty. Each page is independently filtered by the authenticated owner; a cursor
never grants access and foreign or unknown cursors receive the same generic
invalid-request response.

Occurrence cursors are opaque `YYYY-MM-DD~UUID` values because pagination uses
the pair `(due_on, id)`. Clients must pass `next_cursor` unchanged, retaining the
same filters across pages. A cursor can outlive changes to a row or filter; it
identifies only a position, not a frozen export.

Rule items use the existing mutation response fields, including exact decimal
string amounts. Occurrence items expose captured planning terms and an explicit
nullable `transaction_id`; owner IDs and audit timestamps are never returned.
Listing or reviewing a planned occurrence does not post an expense, movement,
or audit record.

See [`contracts/openapi.yaml`](../../contracts/openapi.yaml) and the
[wire examples](../../contracts/examples/recurrence-listing.json) for schemas.
