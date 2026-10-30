# Record income without an account

**Roadmap:** P05.08 · **Status:** Implemented locally; native review and publication pending

`POST /v1/transactions` records one realized ordinary income. It creates no
account movement and never infers a cash account. Reporting can include the
posted event once in its currency and business-date period; transaction queries
and report endpoints remain later tasks.

## Request

Requires a valid bearer, verified email, a confirmed profile IANA zone and one
`Idempotency-Key` header. Use 1–128 ASCII letters, digits, hyphens or underscores,
with the same key on retries. Keys are scoped to the owner and this operation,
not shared with account creation, and have no expiry.

```json
{
  "kind": "income",
  "amount": "300.00",
  "currency": "SEK",
  "occurred_on": "2024-02-29",
  "account_id": null,
  "category_id": null,
  "description": "Synthetic income"
}
```

- Required: `kind` exactly `income`, positive exact decimal-string `amount`,
  `currency` SEK/EUR/USD, and explicit `occurred_on`. Amounts use the existing
  Money grammar, at most two fractional digits, maximum 92233720368547758.07;
  there is no rounding or binary floating-point conversion.
- Dates follow [business-date rules](dates-and-time-zones.md): strict
  `YYYY-MM-DD`, years 0001–9999, no later than today in the confirmed zone.
- `account_id` must be absent or null; linked income belongs to P05.09.
- Optional `category_id`: null or a canonical hyphenated UUID identifying an
  existing income category. UUID hex case is normalized to lowercase.
- Optional `description`: null or up to 1000 Unicode scalar values, with no
  control characters. Preserve text exactly, including whitespace/empty strings.
- JSON is limited to 8192 bytes. Unknown/duplicate fields, nonempty query strings,
  client ownership/state/audit values and transfer fields are rejected.

## Result and retries

Success returns **201** with `id`, `kind`, `state`, `amount`, `currency`,
`occurred_on`, `account_id`, `category_id`, `description`. The amount has two
fractional digits; `account_id` is null and `state` is posted. All application
responses include the existing request ID and `Cache-Control: no-store`.

Equivalent retries return the same original 201 result, without a new event or
audit. Money formatting is normalized; optional null and absence are equivalent;
description text is not normalized. A changed amount, date, currency, category or
description with the same key returns **409 idempotency_conflict**. A fresh key
means a distinct intended income even when its values are identical.

Retry equivalence survives later changes to the event or profile zone. Bearer
validity and current email verification still apply. A replay does not validate
an accepted historical date against a new zone. Failed validation consumes no
key. An ambiguous commit/connection result requires retrying the same key, never
generating another key merely because a response was lost.

The operation locks the owner, validates financial authority and dates, then
atomically inserts the event, a minimal `create` audit with empty changed-field
names, and a permanent retry record. Store only key/request SHA-256 digests and
the owned transaction reference in retry storage, not copies of descriptions or
amounts. Reconstruction from equivalent input preserves the original response
without depending on mutable event fields. There are no movements to reverse.

## Safe errors

| Status | Code |
| --- | --- |
| 400 | `invalid_request`, `invalid_business_date`, `date_out_of_range`, `future_business_date`, `unsupported_time_zone` |
| 401 | `invalid_access_token` |
| 403 | `email_not_verified` |
| 409 | `profile_zone_unconfirmed`, `idempotency_conflict` |
| 413 / 415 | `invalid_request` for oversized body / unsupported JSON media type |
| 503 | `income_creation_unavailable`, `authentication_unavailable`, `request_id_unavailable` |

Rejected operations have no financial/key effects. Database diagnostics and
submitted values are never returned. Verification/recovery runtime remains
pending: this API does not provide a verification bypass or complete journey.
