# Confirm an actual recurring payment (P07.10)

`POST /v1/recurring-occurrences/{id}/confirm` turns an existing planned occurrence
into one posted expense. It records an actual payment, not a payment instruction.
The [lifecycle](recurrence-lifecycle.md) remains authoritative; skip/postpone are
not implemented by this endpoint.

## Request and result

Send a bearer session, an `Idempotency-Key` and a JSON object:

```json
{
  "amount": "123.45",
  "currency": "SEK",
  "occurred_on": "2024-02-29",
  "account_id": null,
  "category_id": null,
  "description": "Actual payment"
}
```

Amount, currency and occurred_on are required actual terms; no planned field is
silently copied. Optional account/category/description default to null. The actual
currency and account need not match the planned ones. Existing ordinary-expense
validation applies: positive exact decimal strings, SEK/EUR/USD, valid nonfuture
business date in the confirmed profile IANA zone, owned active account in the
actual currency, expense category, and bounded description without controls.
Either Normal or Savings may fund an expense. An archived planned account can
be replaced with an active account, or omitted for an accountless payment.

The endpoint accepts at most 8192 JSON bytes, no query parameters, unknown or
duplicate fields. UUIDs normalize hex case; key syntax is 1–128 ASCII letters,
digits, hyphens or underscores, exactly one header. See [OpenAPI](../../contracts/openapi.yaml).
A successful initial call or matching retry returns 201 and the same ordinary
expense representation, with explicit nullable fields. Responses are no-store
and carry server request IDs. Authentication precedes payload validation.

## Atomicity and retries

The verified owner is locked first, then the owner-scoped occurrence and any
actual account. One transaction creates the expense, optional negative account
movement, minimal create audit, confirmation link and permanent retry record.
Accountless expenses affect realized expense reporting but create no movements.
Scheduled date, due date and planned snapshots never change. Existing occurrences
remain actionable even if their rule is paused or ended.

Confirmation keys have their own owner-wide namespace, independent of ordinary
expense creation, rules and transaction corrections. Migration
`0019_recurring_confirmation_keys.sql` stores only key/request hashes and immutable
owner/occurrence/expense identities, not financial value snapshots. Its composite
foreign key binds the retry to the occurrence's exact linked expense.

Same key, occurrence and normalized actual terms replay the original response,
even after the expense is corrected/voided, the account is archived or the profile
zone changes. The caller must still be authenticated and email-verified. Changed
intent or reusing a key for a different owned occurrence returns 409
`idempotency_conflict`. A new key for a confirmed/skipped item returns 409
`occurrence_state_conflict`. Neither corrections nor retries reopen an occurrence.
Concurrent attempts serialize; failures/cancellation roll back every related write.

## Rejections and checks

| Status | Codes / meaning |
| --- | --- |
| 400 | invalid_request, invalid_business_date, date_out_of_range, future_business_date, unsupported_time_zone |
| 401 / 403 | Invalid access token / email_not_verified |
| 404 | Missing or foreign occurrence/account, without ownership disclosure |
| 409 | profile_zone_unconfirmed, idempotency_conflict, occurrence_state_conflict |
| 413 / 415 | Oversized / unsupported media body |
| 503 | recurrence_unavailable, authentication_unavailable, request_id_unavailable |

A rejection changes no occurrence, financial, audit or retry row. Errors never
contain database diagnostics. Shared rule error messages remain unchanged.

Run the dedicated cases against an isolated database:

```bash
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_recurring_confirmation_against_disposable_postgres -v
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_confirmation_key_migration_preserves_history_and_exact_linkage -v
```

Ordinary Cargo tests intentionally ignore these database cases. See the
[execution record](../plans/active/2026-11-01-confirm-occurrence.md) for observed
proof; P07.11–P07.12 and a usable mobile payment calendar remain separate work.
