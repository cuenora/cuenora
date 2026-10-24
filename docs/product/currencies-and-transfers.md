# MVP currencies and transfers

**Roadmap:** P00.06 · **Status:** Accepted · **Date:** 2026-10-22

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

## Worked acceptance examples

Examples are illustrative. Each row starts independently with source A at
1,000 SEK and destination B at 100 EUR, except the explicit same-currency case.

| Scenario | Expected result |
| --- | --- |
| Send 200 SEK and receive 18 EUR, with no rate entered. | A: 800 SEK; B: 118 EUR. One transfer; no income or spending. |
| Enter 0.09 EUR per SEK for 200 SEK sent. | The received amount can be calculated as 18 EUR before confirmation. Confirming these amounts produces A: 800 SEK; B: 118 EUR. |
| Adjust the received amount to 17.95 EUR before confirming. | A: 800 SEK; B: 117.95 EUR. The optional rate cannot override the confirmed amounts. |
| Retry the confirmed 200 SEK / 18 EUR transfer. | A: 800 SEK; B: 118 EUR, with no additional postings. |
| Fail after the first posting of that transfer. | A: 1,000 SEK; B: 100 EUR. Neither posting persists. |
| Transfer 200 SEK between SEK accounts starting at 1,000 SEK and 500 SEK. | Balances become 800 SEK and 700 SEK; the SEK total remains 1,500 SEK. |

## Implementation boundary

This is an accepted product decision, not implemented financial behavior or
executed financial tests. P06.01 must define rate precision, validation, and how
to present a rate that differs from the ratio of the final amounts before code
is written. No rate is inferred as a market quote, no fee is inferred from a
difference, and automatic FX remains outside the MVP.
