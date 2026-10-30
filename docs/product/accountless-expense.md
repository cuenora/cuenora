# Accountless expense creation

P05.10 extends authenticated `POST /v1/transactions` with `kind: "expense"`.
It records spending without inferring a cash account, creating movements or
changing account balances. Reporting endpoints remain pending.

```json
{"kind":"expense","amount":"200.00","currency":"SEK","occurred_on":"2024-02-29","account_id":null}
```

Use exactly one `Idempotency-Key` header (1–128 ASCII letters, digits, `_` or `-`).
A successful request returns 201 with ID, kind, posted state, normalized amount,
currency, date, null account, optional category and description. Responses use
`Cache-Control: no-store` and the existing request-ID/error envelope.

## Validation and authorization

Requires a current bearer session, verified email and a confirmed IANA profile
zone. Dates are explicit, strict Gregorian dates in years 0001–9999, past/today
in that zone. Amounts are positive exact decimal strings in SEK/EUR/USD, at most
92233720368547758.07; excess precision is rejected, never rounded.
`account_id` must be absent/null. An optional category UUID must identify a global
expense category; malformed, missing or income categories are rejected. Optional
description preserves exact text, at most 1000 characters without controls.
Unknown/duplicate JSON fields, nonempty queries and bodies above 8192 bytes are rejected.

## Retry and failure behavior

Permanent keys are scoped by owner and operation: the same key can independently
create an income and an expense. Equivalent normalized expense requests replay
the original 201 with no writes, even after event/profile changes. Current
verification still applies; replay does not require reconfirming the zone.
Null/absence normalize; description bytes remain significant. Different input
with an existing expense key returns 409 `idempotency_conflict`.

Event, empty-changed-field create audit and key record commit atomically under
the owner lock. No movement is created. Late failure rolls everything back;
503 `expense_creation_unavailable` is safe and contains no storage details.
An uncertain commit response should be retried with the same key. Keys never
expire and cannot be updated/deleted/truncated. See [ADR-030](../architecture/decisions/030-accountless-expense-retries.md).

Linked expenses, transaction listing, editing and voiding remain later tasks.
