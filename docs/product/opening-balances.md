# Dated opening balances

**Roadmap:** P05.03 · **Status:** Implemented locally; own/independent checks and native review passed; publication pending

An opening balance is a special signed opening event and append-only account
movement, **never ordinary income**. It is created atomically with a new account.
No opening posting is created when the amount is zero or the opening is omitted.

## Confirm the profile zone

Send an authenticated `PATCH /v1/me` with strict JSON:

```json
{"time_zone":"Europe/Stockholm"}
```

This explicit request confirms or updates that user's profile zone. A pending-email
user may configure it, but that grants no financial authority. Return 200 with
only `{"time_zone":"Europe/Stockholm"}`. The server derives ownership from bearer
identity; unknown/duplicate fields, malformed JSON and bodies over 8192 bytes are
rejected. Numeric offsets, unknown zones, whitespace and wrong-case identifiers
are rejected. Supported IANA aliases are accepted and the submitted identifier is
preserved. `Etc/UTC` is an explicit choice, never a fallback.

Same-value retries do not change `updated_at`. Zone updates serialize on the same
owner row as account creation. Travel/device settings cannot change the saved zone.
This route is not general profile CRUD; no GET profile operation is introduced.

## Create the account with an opening

Extend the existing strict `POST /v1/accounts` request with an optional object:

```json
{
  "name":"Daily",
  "account_type":"normal",
  "currency":"SEK",
  "opening_balance":{"amount":"1250.00","occurred_on":"2026-10-01"}
}
```

Both opening fields are required when the object is present; explicit null is
rejected. `amount` is a decimal string parsed by the existing exact `Money` policy:
no exponent/locale/plus sign, at most two decimal places, no rounding, supported
signed minor-unit bounds. Both normal and savings accounts may have positive,
negative or zero openings. The account's currency supplies the opening currency;
an extra nested currency/owner/category/balance field is invalid.

`occurred_on` must be exact Gregorian ASCII `YYYY-MM-DD`, years 0001–9999, no later
than calendar today in the confirmed profile zone. One trusted server UTC instant
is captured for validation; neither UTC's day alone, database-session zone nor
client/device time defines today. There is no implicit date in this API.

Omit the entire opening object to retain P05.02 behavior: create metadata without
requiring a zone or posting anything. If an explicit opening object is supplied,
even with zero amount, its declared date and confirmed zone are validated and
retained for retry equivalence. Zero still produces no financial event, movement
or transaction audit. Nothing silently ignores a supplied date.

The response remains the original five-field 201 account metadata, with no balance
or date added. Account creation and posting still require current verified email.
Email verification delivery/confirmation remains unimplemented, without a bypass.

## Atomicity and retries

For nonzero amounts, one SQL transaction inserts the owned account, a posted
`opening_balance` event, one signed movement, a create audit with owner-as-actor
and an empty changed-field list, and the lifetime creation key/result. The audit
contains no financial values or snapshots. Any late failure rolls everything back.
The event has no income/expense category and is not ordinary income/spending.

Idempotency now compares normalized name/type/currency **and opening minor amount
and explicit date**. Different decimal spellings of the same exact amount are
equivalent. Changed amount/date or adding an opening to an old no-opening key
returns 409. Existing P05.02 keys migrate as zero/no-date with their original result
intact. Lifetime retention and owner scoping are unchanged.

Matching retries recheck current email verification but never repost or reject an
already accepted date just because the profile zone/today changed. A fresh key
with that now-future date is rejected. Account/event/movement/audit state remains
unchanged by profile-zone changes. After an uncertain commit/network outcome,
retry the same key; never switch keys to guess whether the first write committed.

## Safe errors and boundaries

All responses use the existing server request-ID and no-store policies, including
existing authentication and request-ID-entropy failure handling.

| Condition | Status / code |
| --- | --- |
| Unsupported named zone | 400 `unsupported_time_zone` |
| Profile update persistence failure | 503 `profile_update_unavailable` |
| Explicit opening with no confirmed zone | 409 `profile_zone_unconfirmed` |
| Malformed/impossible date | 400 `invalid_business_date` |
| Date/year or local-date conversion out of range | 400 `date_out_of_range` |
| Future date on a new operation | 400 `future_business_date` |
| Invalid money/body/key | Existing 400 `invalid_request` |
| Different content for an owned key | Existing 409 `idempotency_conflict` |
| Account/posting transaction failure | Existing 503 `account_creation_unavailable` |

No client values, zone details or database errors are echoed. Rejections do not
update financial or audit records. Opening creation is the only new posting use
case here; balance queries, listing, reports, transfers and corrections remain
separate tasks. The known event kind lets later reports exclude openings; this
slice does not implement those reports or claim realized reporting is complete.

## Implementation and verification

The pure `BusinessDate` wrapper narrows Chrono to the project's strict syntax and
range. Chrono-tz's compiled IANA catalog supplies named-zone validation/conversion;
there is no OS-local-zone inference. See [ADR-024](../architecture/decisions/024-dated-opening-balances.md).
Migration `0011_opening_creation_keys.sql` extends existing key records without
expiry or result replacement. Apply migrations transactionally in order; populated
financial records require controlled forward repair/restore, not dropping tables.

The disposable harness includes profile confirmation, opening HTTP tests and an
upgrade test with pre-existing creation keys. It checks signed/zero/omitted cases,
invalid/future dates/zones, owner gates, changed/retried content, concurrent calls,
rollback at each write, and replay after a zone change makes the date future.
Tests seed email verification explicitly; that is not the missing verification
journey. Current evidence and remaining checks are in the
[execution plan](../plans/completed/2026-10-29-opening-balance.md).
