# MVP currencies and transfers

**Roadmap:** P00.06, P06.01 · **Status:** Accepted contract; no-quote creation implemented locally
**Dates:** Initial policy 2026-10-22; rate contract approved 2026-10-31

The MVP supports SEK, EUR, and USD. Users may record transfers between their own
accounts in the same or different supported currencies. This supersedes the
initial same-currency-only MVP restriction, following maintainer approval.

## Currency rules

- Each account has one currency; an assigned income or expense must match it.
- Accountless entries also have an explicit supported currency.
- Balances and reports remain grouped by currency. A transfer rate does not
  authorize converting all balances or displaying a combined cross-currency total.
- Use exact monetary representations. The [money specification](money.md) defines
  scale, bounds and rounding, implemented and tested in P03.01–P03.02.

## Transfer rules

- Record the source account, destination account, business date, amount sent in
  the source currency, and amount received in the destination currency.
- For different currencies, both amounts are required. The user may enter a rate
  to help calculate the received amount, then adjust that amount before confirming.
  A rate is optional when both amounts are entered directly.
- An optional recorded rate is quoted as destination currency units per one
  source currency unit, with both currencies visible. The final sent and received
  amounts determine the postings; a rate must not silently overwrite them.
- Same-currency transfers retain equal sent and received amounts. Fees, if
  supported later, are separate expenses rather than an unexplained difference.
- A transfer is one operation with a debit for the sent amount and a credit for
  the received amount. Both postings succeed together, exactly once on retries.
  Cross-currency postings have opposite signs but cannot be summed across units.
- Only distinct accounts owned by the authenticated user are allowed. Invalid
  amounts, unsupported currencies, and invalid account relationships must leave
  both balances unchanged.
- Transfers do not count as income or spending. Editing or reversing them must
  preserve an auditable history and the original recorded amounts and currencies.
- Cuenora records transfers performed by the user; it does not execute bank
  payments, retrieve rates, or perform automatic currency exchange.

## Recording contract (P06.01)

This defines the full required Phase 06 behavior. The current
[creation slice](transfer-creation.md) supports actual amounts without a quote;
optional rate input/storage and discrepancy presentation remain pending P06.06.

| Input | Required meaning and validation |
| --- | --- |
| Source and destination | Two distinct, active accounts owned by the authenticated, verified user. Normal and savings accounts are allowed. Never accept a caller-selected owner. |
| Business date | Required business date under the confirmed profile-zone and future-date rules in [date semantics](dates-and-time-zones.md); keep it separate from UTC audit timestamps. |
| Sent amount and currency | Required positive recorded amount in the source account's currency. |
| Received amount and currency | Required positive recorded amount in the destination account's currency, including same-currency transfers. |
| Optional quoted rate | Exact positive decimal string, destination units per one source unit, with at most 12 fractional digits. Omission means no quote was recorded; do not invent one from the amounts. |

Both amounts follow the [money contract](money.md): JSON decimal strings, at most
two fractional digits, maximum `92233720368547758.07`, no binary floats and no
silent input rounding. Only uppercase `SEK`, `EUR` and `USD` are supported, and
each currency must match its account. Same-currency transfers require equal
amounts even when a quote is supplied. Negative resulting balances are allowed;
this is a record of a completed transfer, not bank payment authorization.

### Exact optional rate

Accept ASCII `[0-9]+(\.[0-9]{1,12})?` with a value strictly greater than zero.
Leading zeros are allowed; canonical presentation removes unnecessary leading
integer zeros and trailing fractional zeros, including an empty decimal point.
For example, `000.0900` represents `0.09`. Reject more than 12 fractional digits
even when the extra digits are zeros; never round an entered quote to fit.
Reject zero, negative values, signs, whitespace, commas, exponent notation,
missing integer/fractional digits, JSON numbers and non-finite values.

The quote direction is explicit: `0.09 EUR per SEK` for a SEK source and EUR
destination. Never silently invert it. The 12-digit fractional precision is a
recording limit, not a guarantee of market accuracy. Calculations must remain
exact and checked; any representational overflow fails without partial effects.
No database rate type, dependency or migration is introduced by this contract.

### Discrepancy and confirmation

The maintainer approved an **informational warning**, not rejection, for a valid
quote that disagrees with the confirmed amounts. Preserve the quote and both
actual amounts independently; no inferred fee, replacement amount or market-rate
claim is permitted.

1. Calculate the indicative received amount as sent amount × quoted rate using
   exact arithmetic. Quantize once to destination minor units with the existing
   half-even rule; do not round intermediate values.
2. Compare that indicative amount with the actual received amount. A difference
   after that quantization triggers the warning; sub-minor-unit rounding alone
   does not. No percentage tolerance hides a one-cent difference.
3. Present the entered quote with its direction, actual sent/received amounts,
   indicative received amount and signed difference (actual minus indicative)
   in the destination currency. For example: "The recorded rate implies
   18.00 EUR; the confirmed received amount is 17.95 EUR (difference: -0.05 EUR).
   The confirmed amounts will be recorded."
4. Confirmation uses the actual amounts even after the warning. Without a quote,
   there is no quote-discrepancy warning. A same-currency quote does not relax the
   equal-amount rule; if it implies another amount, it remains informational.

A UI may help calculate a received amount before confirmation, but the recording
request still supplies both final amounts. An effective ratio derived for display
must be identified as derived, never substituted for an entered quote.

### Rejection and atomic effects

Reject invalid amounts/rates, missing required amounts, unsupported or mismatched
currencies, identical accounts, inaccessible or archived accounts, invalid dates,
and missing identity/verification/zone prerequisites. Use privacy-safe errors;
never disclose another owner's account existence. A valid quote discrepancy alone
is not a validation error.

Every rejection leaves financial state unchanged: no transfer event, debit,
credit, audit entry or successful idempotency result may persist. If any write
fails, roll back the entire operation. Acceptance records one transfer with a
source debit of the actual sent amount and destination credit of the actual
received amount, atomically with its minimal audit. Retries must not duplicate
these effects; the concrete permanent-key contract and concurrency proof belong
to P06.07–P06.09, not this documentation task.

## Worked acceptance examples

These are specified cases for future implementation tests, **not executed API
acceptance tests**. Each row starts independently with source A at 1,000 SEK and
destination B at 100 EUR, except explicit same-currency cases. Every rejection
also preserves events, movements, audits and successful idempotency results.

| Scenario | Expected result |
| --- | --- |
| Send 200 SEK and receive 18 EUR, with no rate entered. | A: 800 SEK; B: 118 EUR. One transfer; no income or spending. |
| Enter 0.09 EUR per SEK for 200 SEK sent. | The received amount can be calculated as 18 EUR before confirmation. Confirming these amounts produces A: 800 SEK; B: 118 EUR, without a discrepancy warning. |
| Confirm 200 SEK / 17.95 EUR with quote 0.09 EUR per SEK. | Informational warning: indicative 18.00 EUR, difference -0.05 EUR. A: 800 SEK; B: 117.95 EUR. Preserve the quote; it cannot override actual amounts. |
| Confirm 200 SEK / 18.01 EUR with quote 0.09 EUR per SEK. | Warning with difference +0.01 EUR; accept actual amounts, credit 18.01 EUR. |
| Send 1 SEK at 0.125 EUR per SEK; confirm 0.12 EUR. | Exact midpoint quantizes half-even to 0.12 EUR; no warning. Confirming 0.13 EUR instead warns but remains valid. |
| Send 1 SEK at 0.135 EUR per SEK; confirm 0.14 EUR. | Half-even yields 0.14 EUR; no warning. |
| Enter quote `0.000000000001` or `000.0900`. | Valid positive quotes; retain exact values, canonically `0.000000000001` and `0.09`. Actual positive amounts remain required. |
| Enter `0`, `-0.09`, `+0.09`, ` 0.09`, `0,09`, `9e-2`, `.09`, `0.`, `NaN`, or a JSON number. | Reject invalid quote; no financial effects. |
| Enter `0.1234567890123` or `0.0900000000000`. | Reject excessive rate scale, including trailing-zero excess; no rounding or financial effects. |
| Omit either amount; enter zero, a negative amount, `18.001`, or `92233720368547758.08`. | Reject missing/invalid/out-of-range amount; no financial effects. |
| Use `GBP`, lowercase `eur`, or a currency that differs from its account. | Reject; no financial effects. |
| Use the same account twice, another owner's account, or an archived account. | Reject without partial effects or disclosure of another owner's account. |
| Fail after the first posting of the 200 SEK / 18 EUR transfer. | A: 1,000 SEK; B: 100 EUR. Neither posting, event nor audit persists. |
| Retry the confirmed 200 SEK / 18 EUR transfer. | A: 800 SEK; B: 118 EUR, with no additional postings. |
| Transfer 200 SEK between SEK accounts starting at 1,000 SEK and 500 SEK. | Balances become 800 SEK and 700 SEK; the SEK total remains 1,500 SEK. |
| Send 200 SEK and receive 199.99 SEK, with or without a quote. | Reject unequal same-currency amounts; no financial effects. |
| Send 200 SEK from an account holding 100 SEK and receive 18 EUR. | Accept; source becomes -100 SEK. No insufficient-balance rejection. |

## Implementation boundary

P06.01 completed the recording contract and future acceptance cases only. Existing
[transaction storage](../architecture/transaction-storage.md) retains separate
sent/received amounts but has no quoted-rate column. Existing transaction edit/void
behavior alone does not constitute transfer creation. P06.03/P06.07 now add
[no-quote creation and permanent retries](transfer-creation.md), reflected in
OpenAPI. Rate storage and request/response representation remain P06.06.

P06.02–P06.09 implement and prove the creation/posting, authorization, cross-currency,
idempotency and rollback requirements. P06.10 proves reporting exclusion. No rate
is inferred as a market quote, no fee is inferred from a difference, and automatic
FX remains outside the MVP. This contract does not complete Phase 06.
