# Expense creation

P05.10 extends authenticated `POST /v1/transactions` with `kind: "expense"`.
It records spending without inferring a cash account, creating movements or
changing account balances. P05.11 also accepts an optional owned active account
and posts exactly one negative movement. Reporting endpoints remain pending.

```json
{"kind":"expense","amount":"200.00","currency":"SEK","occurred_on":"2024-02-29","account_id":null}
```

Use exactly one `Idempotency-Key` header (1–128 ASCII letters, digits, `_` or `-`).
A successful request returns 201 with ID, kind, posted state, normalized amount,
currency, date, normalized account UUID (or null), optional category and description. Responses use
`Cache-Control: no-store` and the existing request-ID/error envelope.

## Validation and authorization

Requires a current bearer session, verified email and a confirmed IANA profile
zone. Dates are explicit, strict Gregorian dates in years 0001–9999, past/today
in that zone. Amounts are positive exact decimal strings in SEK/EUR/USD, at most
92233720368547758.07; excess precision is rejected, never rounded.
`account_id` may be absent/null, or a canonical hyphenated UUID (hex case normalizes
lowercase) identifying an owned active normal or savings account in the same currency.
Unknown/foreign accounts return the same generic 404; archived or wrong-currency
accounts return 400 `invalid_request`. Insufficient recorded funds do not block
expenses: balances may become negative. An optional category UUID must identify a global
expense category; malformed, missing or income categories are rejected. Optional
description preserves exact text, at most 1000 characters without controls.
Unknown/duplicate JSON fields, nonempty queries and bodies above 8192 bytes are rejected.

## Retry and failure behavior

Permanent keys are scoped by owner and operation: the same key can independently
create an income and an expense. Equivalent normalized expense requests replay
the original 201 with no writes, even after event/profile/account changes. Current
verification still applies; replay does not require reconfirming the zone.
Null/absence normalize; description bytes remain significant. Linked fingerprints
include the normalized account UUID; changing/removing the linked account conflicts.
All pre-existing accountless fingerprints remain unchanged. Different input
with an existing expense key returns 409 `idempotency_conflict`.

Event, optional negative movement, empty-changed-field create audit and key record
commit atomically under owner→account locks, shared with account management and
income posting. Accountless expenses create no movement. Matching retries run
before mutable account checks and never post again. Late failure rolls everything back;
503 `expense_creation_unavailable` is safe and contains no storage details.
An uncertain commit response should be retried with the same key. Keys never
expire and cannot be updated/deleted/truncated. See [ADR-030](../architecture/decisions/030-accountless-expense-retries.md).

Transaction listing, editing and voiding remain later tasks.
