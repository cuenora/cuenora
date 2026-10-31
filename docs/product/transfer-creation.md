# Atomic transfer creation and permanent retries

**Roadmap:** P06.03, P06.07; P06.02 acceptance coverage.
**Status:** Implemented locally; own and independent checks passed. Implementation `f8a186f`; [PR #85](https://github.com/cuenora/cuenora/pull/85) open against `main`.

`POST /v1/transactions` accepts `kind: "transfer"` with required actual sent and
received amounts. It records one business operation, not income plus expense.
The [broader transfer contract](currencies-and-transfers.md) also specifies an
optional quote; quote input/storage remains **pending P06.06** and is rejected
in this slice, never silently ignored. Cuenora does not execute bank payments.

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

## Retry contract

- Keys belong to the authenticated owner and transfer-creation namespace,
  independent of income, expense, account creation and mutation keys. No expiry.
- Normalize monetary input, account UUID case and null/absent description before
  hashing. Exact description text remains significant; empty is not null.
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

See [setup](../development/SETUP.md#transfer-creation-acceptance-tests-p0602-p0603-p0607)
and the [execution plan](../plans/active/2026-10-31-atomic-transfers.md) for actual
observed verification. Dedicated quote, broader concurrency/cancellation and
reporting proofs remain subsequent Phase 06 work; this is not phase completion.
