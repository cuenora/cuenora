# Skip or postpone a planned occurrence (P07.11)

These actions change planning only. They never create an expense, account
movement or transaction audit. Original scheduled identity, planned terms and
rule calendar stay unchanged. See the [lifecycle](recurrence-lifecycle.md).

## Request and result

Use a bearer session and exactly one `Idempotency-Key` header:

| Action | Endpoint | JSON body | Result state |
| --- | --- | --- | --- |
| Skip | `POST /v1/recurring-occurrences/{id}/skip` | `{}` | `skipped` |
| Postpone | `POST /v1/recurring-occurrences/{id}/postpone` | `{"due_on":"2024-02-05"}` | `planned` |

A new action requires an email-verified owner and a planned occurrence.
Existing items remain actionable when their rule is paused or ended, or their
planned account is archived. No profile zone is required: neither action derives
a financial date from today or posts a payment.

Postponement accepts valid Gregorian dates in years 0001–9999 **strictly later
than the current due date**. Equal/earlier dates reject. A date may still be in the
past relative to today and may exceed the rule end. The maintainer approved this
policy on 2026-11-01. Only due_on and server-owned updated_at change; the original
scheduled_on and next rule slot never move. Multiple slots may share one due date.
Skipping is terminal; reopening skipped or confirmed items is not supported.

Both actions return 200 with the original action result, including on retry:

```json
{
  "id": "00000000-0000-0000-0000-000000000041",
  "scheduled_on": "2024-01-31",
  "due_on": "2024-02-05",
  "state": "planned"
}
```

This is an action result, not a current occurrence read. A replay of a postponed
item still returns its original planned result after a later postpone, skip or
confirmation; it does not undo the later action.

## Atomicity and retries

Lock the verified owner first, then the owner-scoped occurrence, using the same
serialization boundary as generation, rule edits and confirmation. Commit the
planning change and permanent retry record in one transaction. Rejections,
failures and cancellation leave no partial writes.

Skip and postpone share one owner-wide key namespace, separate from rule creation,
confirmation and ordinary transactions. A matching key, occurrence, action and
normalized date replays its original result without updating timestamps. Changed
intent or another owned occurrence with that key returns 409 idempotency_conflict.
A new key against a terminal item returns 409 occurrence_state_conflict. A foreign
or missing occurrence always returns 404 without revealing retry information.

Migration `0020_recurring_occurrence_action_keys.sql` adds immutable retry rows
with owner/occurrence linkage, request/key hashes, action and original result due
date. It stores no financial value snapshots and does not rewrite existing rows.

## Validation and errors

No query parameters, unknown/duplicate fields or extra skip fields are accepted.
JSON is required, bounded to 8192 bytes. UUID hex case is normalized; key syntax
is 1–128 ASCII letters, digits, hyphens or underscores, exactly one header.
Authentication precedes payload validation. Responses are no-store with request IDs.

| Status | Codes |
| --- | --- |
| 400 | invalid_request (including non-later due dates), invalid_business_date, date_out_of_range |
| 401 / 403 | invalid_access_token / email_not_verified |
| 404 | not_found for missing/foreign occurrence |
| 409 | idempotency_conflict, occurrence_state_conflict |
| 413 / 415 | invalid_request for oversized body or unsupported media type |
| 503 | recurrence_unavailable, authentication_unavailable, request_id_unavailable |

No storage diagnostics are exposed. Existing confirmation error messages and
expense semantics remain unchanged. See [OpenAPI](../../contracts/openapi.yaml).

## Verification

```bash
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_recurring_occurrence_actions_against_disposable_postgres -v
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_occurrence_action_key_migration_preserves_history -v
```

These use synthetic disposable PostgreSQL only. Ordinary Cargo runs intentionally
ignore database cases. Observed evidence belongs in the
[execution record](../plans/active/2026-11-01-occurrence-actions.md).
