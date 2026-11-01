# Recurrence rule API (P07.07)

Verified owners can create, edit, pause and resume recurring **expense planning**.
These routes never generate occurrences or post money. See the
[lifecycle](recurrence-lifecycle.md), [calendar](recurrence-calendar.md) and
[OpenAPI contract](../../contracts/openapi.yaml).

## Requests

All routes require a bearer session and verified email, JSON bodies of at most
8192 bytes, no query parameters and no unknown/duplicate fields. Success and
errors use `Cache-Control: no-store` and server request IDs. Identity, state,
generation progress and audit timestamps are never client-controlled.

| Method and path | Body | Success |
| --- | --- | --- |
| POST `/v1/recurring-rules` | Complete terms; mandatory `Idempotency-Key` | 201, initial rule |
| PATCH `/v1/recurring-rules/{id}` | One or more term fields | 200, updated rule |
| POST `/v1/recurring-rules/{id}/pause` | `{}` | 200, paused rule |
| POST `/v1/recurring-rules/{id}/resume` | `{}` | 200, active rule |

```json
{
  "amount": "100.00",
  "currency": "SEK",
  "cadence": "monthly",
  "starts_on": "2024-01-31",
  "time_zone": "Europe/Stockholm",
  "account_id": null,
  "category_id": null,
  "description": "Monthly subscription",
  "ends_on": null
}
```

Creation requires amount, currency, cadence, starts_on and time_zone; nullable
fields may be omitted. Amount is an exact positive decimal string within the
existing Money range, with at most two decimal places. Currency is SEK/EUR/USD.
Cadence is weekly/monthly/quarterly/annual. Dates use years 0001–9999; an optional
end is inclusive and cannot precede the start. Scheduled dates may be future dates.
The explicit rule zone must be supported by the existing IANA parser; the profile
zone is neither required nor used. Description allows at most 1000 Unicode
characters and no control characters. Category must be a built-in expense category.

Optional account selection must be owned, currency-compatible and active; either
Normal or Savings is permitted because this is planning, not ordinary income or
posting. Editing other terms, pausing or resuming retains a previously assigned
account even after it is archived. Explicitly selecting a different archived
account is rejected. Future confirmation must independently recheck posting guards.

Responses contain normalized terms plus server `id`, `state`, nullable
`paused_on` and nullable `generated_through`. Optional term fields are explicit
nulls. No owner, exclusion internals, audit timestamps or derived next due date
are returned. Listing and occurrence generation remain separate tasks.

## Edit boundary and pause history

PATCH leaves omitted fields unchanged; null clears only account_id, category_id,
description and ends_on. It preserves active/paused state and every existing
occurrence, including overdue and already-confirmed items. A no-op leaves
`updated_at` unchanged.

Actual changes to starts_on, cadence or time_zone advance the **inclusive**
`generated_through` boundary to the maximum of its previous value and the civil
edit day in both old and new rule zones. Capture one server instant after acquiring
locks. P07.08 must generate only slots strictly after this boundary: no revised
slot at/before the edit day or already-covered period may appear. The new start
remains the original anchor of the revised schedule, not the boundary itself.
Term-only edits and changes to ends_on preserve progress and do not suppress
ordinary active backlog. They still never rewrite existing occurrence snapshots.

Pause derives its civil date from the rule zone using the captured server instant.
Resume unions `[paused_on, resume_today)` into `excluded_dates`, clears paused_on
and makes the rule active. The resume-day slot remains eligible unless independently
covered/excluded. Same-day pause/resume adds an empty interval. Completed intervals
and the original anchor survive repeat pauses, edits and profile changes. Zone
edits preserve existing civil-date exclusion labels and the open pause date;
a resume before that stored date rejects without mutation. Worker downtime is not
a user pause. Pausing never deletes or hides existing commitments.

## Retry and serialization

Creation keys follow the existing 1–128 ASCII alphanumeric/underscore/hyphen
policy. Permanent owner-scoped SHA-256 key/request hashes live in additive
migration 0018. Fingerprints frame normalized fields with lengths and null markers.
Matching retries reconstruct the original creation representation, even after
edits, archival or pause; changed intent returns 409. A replay is not a current
rule read. Different owners may use the same key independently.

PATCH and pause/resume assign current terms/state, not historical operations.
Repeating a no-op leaves audit time unchanged. Retrying a pause after another
client has resumed is a **new pause**, not a replay of the original pause. Clients
must serialize their own lifecycle actions. All mutations acquire owner → rule →
account locks in READ COMMITTED transactions, with 2-second lock and 5-second
statement timeouts. Creation replays are checked before account eligibility.
Future generators must use the same owner/rule lock order and atomically maintain
progress/exclusions. No extra dependencies or financial value snapshots are added.

## Errors and verification

400 covers malformed terms, invalid dates, unsupported zones and incompatible
currency/category. 401 rejects invalid sessions, 403 unverified owners, 404 hides
foreign/missing rules and accounts. 409 codes are `idempotency_conflict`,
`recurrence_account_archived`, and `recurrence_state_conflict`. Oversized/non-JSON
bodies use 413/415; storage failures use 503 `recurrence_unavailable`. Errors never
include SQL diagnostics and rejected requests change no recurrence/financial data.

Public HTTP tests run against disposable PostgreSQL:

```bash
python3 tests/infrastructure/test_postgres_compose.py \
  PostgresComposeTest.test_recurring_rules_against_disposable_postgres \
  PostgresComposeTest.test_recurring_rule_retry_migration_preserves_history_and_keys -v
```

Ordinary `cargo test --workspace --all-features --locked` intentionally ignores
these database cases; it is not evidence that PostgreSQL behavior passed.
