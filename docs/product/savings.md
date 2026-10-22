# Savings representation and accounting

**Roadmap:** P00.07 · **Status:** Accepted product definition · **Date:** 2026-10-22

Savings means money held in accounts the user designates for savings. An earmarked
part of an everyday account is not a separate savings balance in the MVP.

## Accounts and totals

- A savings account is an owned account with one supported currency, not a second
  copy of money already recorded elsewhere.
- Savings totals are the posted balances of designated savings accounts, grouped
  by currency. Those accounts contribute only once to overall account totals.
- Moving money into or out of savings is a transfer, not spending or income.
  The [currency and transfer rules](currencies-and-transfers.md) also apply here.
- Designating an account for savings does not create money or income. It changes
  which balances are included in the savings view.
- A savings account may earn no interest. Interest settings are optional and do
  not determine whether an account is classified as savings.

## Funding and allowed income

The maintainer confirmed this account-type restriction on 2026-10-22:

- At creation, a savings account may record money already held as an opening
  balance movement. This is neither income nor an invented transfer.
- After opening, contributions enter through transfers from another owned
  account. The source account decreases as the savings account increases.
- The only direct income allowed in a savings account is actual credited
  interest. Ordinary income, such as salary or payment for work, is allowed in
  normal accounts, not in savings accounts.
- Apply this restriction on creation, editing, and later account assignment.
  Linking an accountless ordinary income to savings must fail without changing
  the entry or any balance. Do not automatically relabel it as interest.
- Corrections and reversals preserve history and the original operation's
  meaning; they are not a mechanism for recording new ordinary income.

These are Cuenora product rules, not claims about which deposits a bank accepts.
Use "interest" for these bank savings payouts; investment dividends are outside
this savings-income definition. Interest capitalization means retaining credited
interest in the account, distinct from contributing money by transfer.

## Interest terms

For accounts that earn interest, record the annual percentage separately from
the payout frequency. Support daily, monthly, and annual payouts; retain the
roadmap's planned quarterly option. Daily payment does not imply a daily quoted
rate.

Rates can change. Store a rate's effective date and preserve previous periods
rather than replacing the rate for all history. A change must not recalculate
already posted interest payments.

An annual percentage alone does not identify a nominal or effective annual rate.
The initial calculator targets explicitly identified nominal annual rates, as
in the master plan. Do not silently interpret an unclassified rate or an effective
annual yield as nominal. Account tracking and actual payout recording remain
useful even when terms are insufficient for a forecast.

Before enabling a forecast, P08.01 must specify the supported rate basis, day-count
convention, balance timing, rate-change boundaries, payout schedule, capitalization,
and rounding. Do not infer those terms from a provider's name or payment cadence.
Effective-rate calculation support requires an explicit later decision.

## Forecasts and actual payments

- Forecast interest is an estimate shown separately. It creates no posted
  movement, realized income, or spendable balance, including on an expected
  payout date.
- Record an actual credited payment only after user confirmation. It creates
  interest income and a matching movement exactly once in the receiving account.
- If paid into the savings account, the payment increases that account and the
  savings total. If paid into another owned account, it increases that receiving
  account; it is not also added to the savings account.
- Forecast capitalization must follow explicitly configured terms. Actual
  credited payments affect balances; unconfirmed forecasts never do.
- Corrections preserve auditable history. Actual credited amounts are recorded
  as received, even when they differ from a forecast.

## Worked acceptance examples

Each row is independent. Amounts are illustrative, not bank-specific terms.

| Scenario | Expected accounting result |
| --- | --- |
| Create a savings account with 1,000 SEK already held. | Savings: 1,000 SEK; opening movement: 1,000 SEK; realized income: zero. |
| Attempt to post 200 SEK of work income into savings holding 1,000 SEK. | Reject the operation; savings remains 1,000 SEK; no new entry or movement. |
| Attempt to assign an existing accountless 200 SEK work income to savings holding 1,000 SEK. | Reject the assignment; savings remains 1,000 SEK; the existing income stays accountless and appears once in reports. |
| Everyday account: 3,000 SEK; savings account without interest: 1,000 SEK. | Savings: 1,000 SEK; overall accounts: 4,000 SEK. |
| Transfer 500 SEK from that everyday account to savings. | Everyday: 2,500 SEK; savings: 1,500 SEK; overall: 4,000 SEK; no income or expense. |
| Savings balance: 1,000 SEK; forecast interest: 5 SEK. | Posted savings balance remains 1,000 SEK; realized interest income remains zero. |
| Confirm an actual 4 SEK interest payment into that savings account, then retry. | Savings: 1,004 SEK; interest income: 4 SEK; no duplicate posting. |
| Savings balance: 1,000 SEK; actual interest of 4 SEK paid into an everyday account with 3,000 SEK. | Savings: 1,000 SEK; everyday: 3,004 SEK; interest income: 4 SEK. |
| Nominal annual rate changes from 2% to 3% on a recorded effective date. | Preserve both rate periods; forecasts use the applicable period once calculation rules are defined; posted payouts remain unchanged. |
| Savings accounts hold 1,000 SEK and 100 EUR. | Display savings totals of 1,000 SEK and 100 EUR separately, without implicit conversion. |

## Implementation boundary

This resolves product representation and accounting, not the bank-specific
interest formula. No financial behavior is implemented or tested by this document.
P08.01 and the money rules in P03 must settle calculation policies before the
calculator is implemented. See the
[master plan](../plans/master-plan.md#interest-bearing-savings).
