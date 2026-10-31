# Atomic transfer creation and permanent retries

**Roadmap:** P06.03, P06.06, P06.07; P06.02 acceptance coverage.
**Status:** No-quote creation merged; optional quotes implemented locally in `7dd271a` with own/independent checks passed, not published. Implementation `f8a186f`; [PR #85](https://github.com/cuenora/cuenora/pull/85) merged into `main`.

`POST /v1/transactions` accepts `kind: "transfer"` with required actual sent and
received amounts. It records one business operation, not income plus expense.
P06.06 adds the optional quote from the [broader transfer contract](currencies-and-transfers.md)
locally; verification and publication status are tracked in its
[execution plan](../plans/active/2026-10-31-quoted-transfers.md). Cuenora does not execute bank payments.

## Request and acknowledgement

Use a valid bearer session for a currently verified owner and exactly one
`Idempotency-Key`: 1–128 case-sensitive ASCII letters, digits, `_` or `-`.
The strict JSON body is limited to 8192 bytes; nonempty queries, duplicate or
unknown fields reject. A new posting also requires a confirmed IANA profile zone.

```json
{
  "kind": "transfer",
  "amount": "200.00",
  "currency": "SEK",
  "account_id": "00000000-0000-0000-0000-000000000021",
  "destination_account_id": "00000000-0000-0000-0000-000000000023",
  "received_amount": "17.95",
  "received_currency": "EUR",
  "occurred_on": "2024-02-29"
}
```

Both accounts must be distinct, owned, active and currency-matched; normal and
savings accounts are eligible on either side. Amounts are positive exact decimal
strings with at most two fractional digits, maximum `92233720368547758.07`.
Same-currency amounts must match. Negative resulting balances are allowed.
The business date follows the existing confirmed-zone, Gregorian and no-future
rules. Optional `description` is exact text up to 1000 characters without
controls. Category and caller-supplied authority/balances are not accepted.

`201 Created` returns `id`, `kind: "transfer"`, `state: "posted"`, both amounts
formatted with two fractional digits, currencies, normalized lowercase account
UUIDs, business date, `category_id: null` and the original description or null.
The transaction contains exactly one source debit and destination credit in their
own currencies, plus one minimal create audit with no financial snapshots.
All effects, including the retry key, commit in one SQL transaction.

## Optional entered quote (P06.06)

`quoted_rate` is an optional positive ASCII decimal string with at most twelve
fractional digits, quoted as destination units per source unit. Null and omission
both mean no quote. Canonicalization removes leading integer and trailing
fractional zeros: `000.0900` becomes `0.09`. Zero, signs, whitespace, exponents,
JSON numbers and excessive scale (even trailing zeros) reject without rounding.
The canonical coefficient and its product with sent minor units must fit signed
`i128`; the half-even indicative result must fit the money range. Overflow rejects
with `400 invalid_request`, without consuming the retry key.

With a quote, creation and history include two additional fields:

```json
{
  "quoted_rate": "0.09",
  "rate_comparison": {
    "indicative_received_amount": "18.00",
    "difference": "-0.05",
    "mismatch": true
  }
}
```

For 200 SEK sent and 17.95 EUR confirmed received, the quote implies 18.00 EUR.
The difference is actual minus indicative in `received_currency`. Positive
values have no leading plus sign. Clients use `mismatch` to show an informational
warning with both currencies/amounts and quote direction; creation still posts
200 SEK and 17.95 EUR. Quantization happens once, using checked exact arithmetic
and half-even rounding. A same-currency quote never relaxes equal actual amounts.
No quote means both additional fields are omitted, preserving existing responses.

The entered quote is stored independently of the actual amounts. Existing edits
and voids preserve it; history recomputes the comparison using current amounts,
even for voided events. An edit that would overflow that calculation rejects
without effects. Editing/removing the quote itself is not part of this slice;
`PATCH` continues to reject `quoted_rate` as an unknown field.

## Retry contract

- Keys belong to the authenticated owner and transfer-creation namespace,
  independent of income, expense, account creation and mutation keys. No expiry.
- Normalize monetary input, account UUID case and null/absent description before
  hashing. Canonical quotes are significant: equivalent spellings replay; a changed
  quote or adding/removing one conflicts. No-quote request hashes remain unchanged.
  Exact description text remains significant; empty is not null.
- Equal key and canonical input return the original `201` acknowledgement without
  new postings. Different input under that key returns `409 idempotency_conflict`.
- Replay reconstructs the original acknowledgement from the matching immutable
  request hash and request, not the current event. Later edits, voiding, account
  archival or profile-zone changes cannot change it. Current bearer and email
  verification still apply. No stored financial value snapshots are needed.
- Owner-first locking serializes transfer retries with other financial postings,
  account archive and corrections. Account rows lock in UUID order after the
  owner. A failed/cancelled SQL transaction commits no event, movement, audit or
  successful key. A lost commit response is ambiguous: retry the same key.

## Safe failures

| Status | Code and meaning |
| --- | --- |
| 400 | `invalid_request`: invalid key, fields, amount, currency, same-account relationship, archived account or account/currency mismatch. Existing typed date/zone errors also apply. |
| 401 | Existing `invalid_access_token`; authentication precedes body validation. |
| 403 | `email_not_verified`. |
| 404 | Generic `not_found` for missing/inaccessible accounts, without revealing another owner. |
| 409 | `idempotency_conflict` or `profile_zone_unconfirmed`. |
| 413/415 | Existing body-size/media-type handling. |
| 503 | `transfer_creation_unavailable`: storage/timeout/failure, with no internal diagnostics. |

Failures use the existing request ID, no-store and safe error envelope.

## Storage and verification boundary

Apply `0015_transfer_creation_keys.sql` transactionally after prior migrations.
It adds the independent owner/key/request hashes and owned event reference,
with unique event binding, 32-byte hash constraints and immutable keys. Existing
financial rows are not rewritten. No down migration is provided; use reviewed
forward corrections for populated installations, never delete keys to retry.

Apply `0016_transfer_quotes.sql` transactionally after `0015`. It adds a nullable
canonical-text quote with transfer-only, grammar, coefficient and checked-calculation
constraints; existing records retain null without inferred quotes or rewritten
amounts. No dependency or database floating-point rate type is introduced.

See [setup](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607)
and the [execution plan](../plans/completed/2026-10-31-atomic-transfers.md) for actual
observed verification. Broader concurrency/cancellation and
reporting proofs remain subsequent Phase 06 work; this is not phase completion.
