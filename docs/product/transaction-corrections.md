# Edit and void recorded transactions

**Roadmap:** P05.13 · **Status:** Implemented locally; own/independent checks passed; native review skipped for this candidate by maintainer choice; [PR #83](https://github.com/cuenora/cuenora/pull/83) open

`PATCH /v1/transactions/{id}` edits a recorded event. `POST
/v1/transactions/{id}/void` cancels its financial effect without deleting it.
Both require an active bearer, currently verified email, a canonical UUID,
JSON bounded to 8192 bytes and one `Idempotency-Key` (1–128 ASCII letters,
digits, `_` or `-`). Query parameters, duplicate/unknown JSON fields and
client audit/owner/balance overrides are rejected.

All five stored kinds are supported: income, expense, opening balance,
transfer and credited interest. This does not introduce transfer/interest
creation, predicted payments, rates, or interest calculations.

## Requests

PATCH requires at least one field. Omitted fields retain their latest locked
values. This is a partial assignment, not increment/decrement semantics.

```json
{"amount":"125.50","occurred_on":"2024-03-01","description":"Corrected receipt"}
```

| Field | Rule |
| --- | --- |
| `amount`, `currency` | Exact decimal string and SEK/EUR/USD. Non-openings must be positive; openings may be signed or zero. All account/currency relationships must match the final event. |
| `occurred_on` | Strict Gregorian date in years 0001–9999. Every explicitly supplied date requires a confirmed profile IANA zone and cannot be future relative to one captured UTC instant. Unrelated edits/voids retain historic dates without revalidation after zone changes. |
| `account_id` | Owned matching-currency account. Income requires normal, expense allows normal/savings. Null removes the account only for income/expense; accountless events affect reports, not balances. Openings and credited interest require an account. |
| `destination_account_id`, `received_amount`, `received_currency` | Transfer legs must remain complete, positive and use distinct owned accounts. Same-currency sent/received amounts must match; cross-currency amounts remain independent. Other kinds cannot acquire transfer legs. |
| `category_id` | Existing compatible global income/expense category; interest uses income. Null clears it. Openings/transfers cannot have categories. |
| `description` | Null clears it; otherwise at most 1000 Unicode scalar values, no controls. Empty string remains distinct. |

Kind, ID, owner and creation instant are immutable. An income cannot be relabeled
as interest to bypass savings restrictions. Changing a referenced account is a
correction of the existing event, not another opening or a new deposit.
Nonnullable fields reject null. Transfer nullable fields may be cleared only
when the complete resulting event remains valid.

Void accepts exactly `{}`. A voided event cannot be edited or restored. A second
void is a successful no-op (with its own retry record but no additional audit or
postings). Re-record an operation through its supported creation workflow if a
new business event is needed.

## Result and retries

Success is HTTP 200 with a minimal acknowledgement:

```json
{"id":"00000000-0000-0000-0000-000000000001","state":"posted"}
```

Void returns `state: voided`. Read the current full event through the
[transaction listing](transaction-listing.md). All responses use request IDs and
`Cache-Control: no-store`.

Keys are permanent and owner-scoped in a mutation-only namespace, binding the
target ID, edit/void action and normalized supplied fields. UUID hex case and
exact equivalent decimal spellings normalize; omitted and explicit null remain
different patch instructions. Reusing a key for a different target/action/patch
returns 409. Retrying a matching key returns the original acknowledgement even
after later edits or voiding; it never reapplies an old assignment. Recheck
current email verification on replay, but not later account/profile state.
No financial values or description snapshots are stored in retry records.

Different keys serialize and use the latest event state; there is no optimistic
version/ETag guard. A fresh key can intentionally apply an older assignment.
After an uncertain commit/network result, retry the same key, never invent a
new one. Original income/expense/account creation keys still replay their
original response and never recreate financial effects after a correction.

## Atomic posting and archived accounts

One SQL transaction locks the owner, event and affected accounts (sorted UUIDs,
consistent with owner-first creation/archive locking). Movement-affecting edits
append exact inverse postings linked to the currently effective original
postings, then replacement postings. Inverses retain the original account,
currency and business date; replacements use the edited event's date. A transfer
corrects both legs atomically. All original movements remain unchanged.
Void appends inverses only and changes state; balances sum every movement,
including compensations, without filtering by current event state.

All affected old/new accounts must be active before movement changes. Reactivate
archived accounts first. Description/category-only corrections and unchanged
assignments remain possible on archived accounts and do not append movements.
An accountless amount/date edit has no account movement effect. Even a date-only
correction on a linked zero opening changes its posting history and requires an
active account.

Accepted changes update only the event's modification instant and append one
owner-as-actor audit containing action and actual changed field names. No values,
financial snapshots or descriptions are copied to audit. No-op assignments do
not update timestamps/audit/movements. Every rejected or failed operation rolls
back event, both movement legs, audit and retry records. Malformed legacy posting
sets fail closed rather than inventing a reversal; controlled repair is separate.

## Errors

| Condition | HTTP / code |
| --- | --- |
| Missing/invalid/revoked/expired bearer | 401 `invalid_access_token` |
| Unverified owner | 403 `email_not_verified` |
| Unknown or foreign event/account | 404 `not_found`, indistinguishable |
| Invalid patch, money, relationship, key or query | 400 `invalid_request` |
| Invalid/out-of-range/future explicit date | 400 `invalid_business_date` / `date_out_of_range` / `future_business_date` |
| Missing confirmed zone / unsupported zone | 409 `profile_zone_unconfirmed` / 400 `unsupported_time_zone` |
| Reused key with different input | 409 `idempotency_conflict` |
| Editing a voided event | 409 `transaction_voided` |
| Archived affected account | 409 `account_archived` |
| Body too large / non-JSON media | 413 / 415 `invalid_request` |
| Database/lock/posting consistency failure | 503 `transaction_update_unavailable` |

Safe errors never disclose IDs, account state, amounts, input text or database
diagnostics. See [ADR-032](../architecture/decisions/032-atomic-transaction-corrections.md)
and the [execution plan](../plans/active/2026-10-30-transaction-mutations.md).
