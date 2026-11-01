# Savings representation and accounting

**Roadmap:** P00.07, P08.01 · **Status:** Accepted product and interest policy · **Updated:** 2026-11-01

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

### Approved forecast policies (P08.01)

The maintainer approved these rules individually on 2026-11-01. They define
Cuenora estimates, not provider-specific bank terms. Interest terms remain
optional; missing or unclassified terms must not be silently inferred.

| Concern | Approved policy |
| --- | --- |
| Rate basis | Explicit nominal annual percentage, separate from payout frequency; effective annual yields are not interpreted as nominal. |
| Balance timing | Daily closing posted balance, including movements with that financial date, regardless of entry time or intraday order. |
| Day count | Each actual calendar day uses the annual rate divided by 365 or 366 according to that day's calendar year. Split at January 1 when a period spans years; this is the precise convention meant here by Actual/Actual. |
| Rate changes | A new rate applies from its effective financial date inclusive. Preserve earlier periods. Changes do not restart payout periods or alter already recorded interest. |
| Payout calendar | Daily, calendar month-end, quarter-end, or year-end. A mid-period terms start produces a partial first period. Custom bank payout dates are outside this MVP policy. |
| Rounding | Accumulate without daily rounding; round to currency minor units only at payout-period close, using nearest with ties to even. |
| Capitalization | Explicitly configurable: reinvest the rounded projected payout or exclude it from the calculation base. Reinvestment starts the following day and exists only inside the projection; no default mode is assumed. |
| Negative balance | A negative daily balance generates zero estimated interest; it does not change the posted balance or invent overdraft charges. |
| Rate sign | Zero is permitted; negative nominal rates are rejected in the MVP. Charges require separate rules. This supplies the policy for P08.08, not its implementation or test completion. |

The unrounded daily interest is
`max(daily forecast base, 0) × (nominal annual percentage / 100) / days in calendar year`.
The daily forecast base is the closing posted balance plus any applicable
capitalization retained only inside the projection.
Any projected capitalization is separate from the posted balance. These rules
never authorize double-counting an actual receipt as a projected receipt too.

P08.02 additionally preserves payout-frequency and capitalization changes by
inclusive effective date, alongside independent rate history. Nominal annual
percentage inputs support up to six fractional digits without rounding; excess
precision is rejected. See the [storage contract](../architecture/savings-storage.md).

### Hand-worked policy examples

These are policy examples. P08.04 tests execute the single-day closing-base,
leap-year daily estimate and negative-balance examples. P08.05/P08.06 tests
add full-period, partial-period, payout-rounding and capitalization examples.

| Scenario | Expected forecast result |
| --- | --- |
| Start with 1,000 SEK, transfer in 500 SEK and out 200 SEK on the same financial date; nominal rate 3.65%, non-leap year. | Closing base is 1,300 SEK; that day's unrounded estimate is `1300 × 0.0365 / 365 = 0.13 SEK`. |
| Constant 1,000 SEK at 3.66% throughout a leap year, without capitalization. | Daily estimate is `1000 × 0.0366 / 366 = 0.10 SEK`; full-year unrounded accrual is 36.60 SEK. |
| Rate changes from 2% to 3% effective March 15. | Use 2% through March 14 and 3% from March 15, within the same payout period. |
| Closing balance is -200 SEK at a positive nominal rate. | Estimate zero interest for that day; keep the posted balance at -200 SEK and create no charge. |
| Exact period totals are 1.005 SEK and 1.015 SEK. | Round to 1.00 SEK and 1.02 SEK respectively (ties to even). |
| January closes with a rounded 3 SEK estimate on 1,000 SEK; capitalization enabled. | Projected base becomes 1,003 SEK on February 1; posted balance remains 1,000 SEK without an actual confirmed receipt. |
| Terms start April 20 with monthly payouts. | First period covers April 20–30 inclusive; the next is May 1–31. |

### Scope and limitations

Negative posted balances remain valid under the [money policy](money.md); the
zero floor affects only estimated interest. Effective annual yields, custom bank
payout calendars and overdraft charges are not modeled by these rules. The [P08.03 configuration API](../architecture/savings-storage.md#configuration-api-p0803)
adds runtime terms/rate validation; the pure daily primitive is described below. Provider names
and payment cadence never establish missing terms.

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
| Nominal annual rate changes from 2% to 3% on a recorded effective date. | Preserve both rate periods; forecasts use the applicable rate from its effective date inclusive; posted payouts remain unchanged. |
| Savings accounts hold 1,000 SEK and 100 EUR. | Display savings totals of 1,000 SEK and 100 EUR separately, without implicit conversion. |

## Implementation boundary

This resolves product representation and accounting, not the bank-specific
interest formula. This document itself does not implement financial behavior.
The approved P08.01 policies are inputs to implementation; the policy task itself
delivers no calculator, migration or API. The P08.04 primitive is documented below. See the
[master plan](../plans/master-plan.md#interest-bearing-savings).

## Pure daily calculator (P08.04)

`cuenora-domain::daily_interest` accepts a closing `Money` base, a parsed
`NominalAnnualRate` and a `BusinessDate`. It returns `DailyInterest`: an exact,
reduced numerator/positive denominator in currency **minor units**, not a rounded
`Money` value. For example, 1 SEK at 1% in 2025 returns `1/365` minor units.
The caller selects the rate effective on that date. No database, clock, period
accumulation, payout, capitalization or financial posting is involved.

The rate parser matches configuration: ASCII nonnegative decimal, at most six
fractional digits, maximum 1024 characters, no signs, whitespace or exponents.
Arbitrary-precision fractions preserve repeating results and large accepted rates;
see [ADR-043](../architecture/decisions/043-exact-daily-interest.md).

Run `cargo test -p cuenora-domain --test savings_interest` from `backend/`.
The payout forecast layer below composes this primitive; the daily primitive
itself performs no period scheduling or rounding.

## Pure payout forecasts (P08.05 and P08.06)

`cuenora-domain::forecast_interest` accepts a nonempty ordered slice of
`ForecastDay` values plus explicit `PayoutFrequency` and `Capitalization` enums.
It supports daily, calendar-month-end, quarter-end and year-end payouts, with
`Exclude` or `Reinvest` chosen explicitly (no default).

Callers provide every day's posted closing `Money` balance and applicable
`NominalAnnualRate`, including unchanged days. Dates must be consecutive and
strictly increasing; currencies must match. Missing days, duplicate/reversed
dates, empty input and mixed currencies return typed errors, never partial results.
Effective rates are selected by the caller; a rate change does not restart a period.

The result carries its currency and inclusive period breakdowns: start, end,
exact rational minor-unit accrual, and an optional rounded `BigInt` minor-unit
payout. A partial first period closes on the natural calendar boundary. An open
final period retains its exact accrual **without** inventing an early payout.
Only closed periods round, nearest with ties to even; fractional residuals are
not carried between closed payout periods.

Reinvestment adds only rounded payouts to subsequent days' projected bases.
Arbitrary-precision capital can exceed the recorded `Money` range without
narrowing or overflow. A negative combined posted-plus-projected base accrues
zero. Forecast payouts are estimates, not `Money`, receipts or account movements.
Inputs remain unchanged; no HTTP, database or clock access occurs.

### Caller boundary and limitations

Each invocation uses fixed payout/capitalization terms and starts with zero
projected capital. Supply days from the terms start or a natural period start;
an arbitrary mid-period start intentionally yields only a partial first period.
Do not split a calculation at rate changes or treat a second call as a continuation.
Resolving persisted effective-dated terms changes, carrying accrual/capital across
such changes and fetching daily balances belong to future orchestration, not this
fixed-terms primitive. Avoid overlapping projected payouts with actual receipts
already in supplied posted balances; there is no reconciliation/deduplication here.
No forecast endpoint or actual-interest posting is introduced.

Run `cargo test -p cuenora-domain --test savings_forecast --locked` from `backend/`.
Tests cover all four payout calendars, partial periods, half-even ties,
capitalization from the next day, leap years, rate/balance changes, currencies,
validation, date bounds and arbitrary-precision projected amounts.
