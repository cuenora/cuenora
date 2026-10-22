# Ten numerical worked examples

**Roadmap:** P00.09 · **Status:** Defined · **Date:** 2026-10-22

These examples specify expected product results; they are not executed tests of
implemented financial features. Every example starts independently, all accounts
belong to one user, and all amounts are fictional. Report figures below describe
only the operations in that example, excluding opening balances and transfers.
Unless stated otherwise, operations occur in the same business-date month.

## EX-01 — Opening balances and savings totals

Create normal account A with 3,000 SEK, savings account S with 1,000 SEK, and
normal account U with 100 USD as their opening balances.

- Posted balances: A = 3,000 SEK; S = 1,000 SEK; U = 100 USD.
- Overall totals: 3,000 + 1,000 = **4,000 SEK**, and **100 USD** separately.
- Savings total: **1,000 SEK**, already included in the 4,000 SEK overall total.
- Income = 0; spending = 0 in each currency. Each nonzero opening balance is
  one opening movement, not income or an invented transfer.

## EX-02 — Ordinary income and an assigned expense

Normal account A starts at 1,000 SEK. Record 2,000 SEK of work income, then a
300 SEK grocery expense against A.

- After income: 1,000 + 2,000 = **3,000 SEK**.
- After expense: 3,000 − 300 = **2,700 SEK**.
- Income = **2,000 SEK**; spending = **300 SEK**; net flow = **1,700 SEK**.
- There is one +2,000 SEK movement and one −300 SEK movement beyond the opening.
  The 2,700 SEK balance is distinct from the period's 1,700 SEK net flow.

## EX-03 — Accountless expense, later assignment, and reassignment

Normal accounts A and B start at 1,000 SEK and 500 SEK. Record a 200 SEK expense
without an account, then assign it to A. Retry that assignment, then change the
assignment to B.

- Initially: A = **1,000 SEK**; B = **500 SEK**; spending = **200 SEK**;
  the expense has zero account movements.
- After assigning to A: A = 1,000 − 200 = **800 SEK**; B = **500 SEK**.
- Retrying leaves those balances unchanged and does not duplicate spending.
- Reassigning reverses A's −200 SEK effect and applies it to B atomically:
  A = **1,000 SEK**; B = 500 − 200 = **300 SEK**.
- Spending remains **200 SEK**, income remains zero, and the original business
  date remains unchanged throughout; the correction history is preserved.

## EX-04 — Accountless income and business-date reporting

Normal account A starts at 100 EUR. Record 50 EUR of work income dated
2026-10-20 without an account. Assign it to A on 2026-11-02, then retry.

- Before assignment: A = **100 EUR**; October income = **50 EUR**.
- After assignment and retry: A = 100 + 50 = **150 EUR**, exactly once.
- October income remains **50 EUR**; November income from this operation is
  **0 EUR**. Spending is zero. The assignment timestamp does not replace the
  business date or create a second report item.

## EX-05 — Funding savings by same-currency transfer

Normal account A starts at 3,000 SEK; savings account S starts at 1,000 SEK.
Record a transfer of 500 SEK from A to S.

- A = 3,000 − 500 = **2,500 SEK**; S = 1,000 + 500 = **1,500 SEK**.
- Overall total = 2,500 + 1,500 = **4,000 SEK**, unchanged.
- Savings total = **1,500 SEK**; income = **0 SEK**; spending = **0 SEK**.
- One transfer creates two postings: −500 SEK and +500 SEK. Their sum is zero.

## EX-06 — Cross-currency transfers, optional rate, and actual amounts

Normal accounts A, E, and U start at 1,000 SEK, 100 EUR, and 50 USD.
First record 200 SEK sent from A and 18 EUR received in E, with no rate entered.
Next send 10 EUR from E to U: enter a helper rate of 1.10 USD per EUR, but adjust
the calculated 11 USD received to the actual 10.95 USD before confirming.

- After the first transfer: A = **800 SEK**; E = **118 EUR**; U = **50 USD**.
- After the second: A = **800 SEK**; E = 118 − 10 = **108 EUR**;
  U = 50 + 10.95 = **60.95 USD**.
- Overall totals remain separate: **800 SEK**, **108 EUR**, **60.95 USD**.
- Income and spending remain zero in each currency. The optional rate does not
  overwrite the confirmed 10.95 USD, infer a fee, or convert overall totals.

## EX-07 — Transfer failure and safe retries

Normal account A starts at 1,000 SEK and savings account S at 500 SEK. Attempt a
300 SEK transfer from A to S; force a failure between the two postings. Retry
the operation successfully, then retry the successful request with the same key.

- After failure: A = **1,000 SEK**; S = **500 SEK**; zero transfer postings
  persist. There is no intermediate 700 SEK / 500 SEK committed result.
- After successful retry: A = **700 SEK**; S = **800 SEK**.
- After retrying the success: A = **700 SEK**; S = **800 SEK**; exactly one
  transfer and two transfer postings exist.
- Overall total remains **1,500 SEK**; income and spending remain **0 SEK**.

## EX-08 — A scheduled payment needs funds in its source account

Normal accounts A and B start at 300 SEK and 2,000 SEK. A recurring payment of
800 SEK from A is due on 2026-10-30. For the planning comparison through that
date, this is the only pending payment.

- Before confirmation, balances remain A = **300 SEK**, B = **2,000 SEK**;
  spending = **0 SEK**. Passing the due date alone must not post the payment.
- A's planning difference is 300 − 800 = **−500 SEK**, even though the combined
  posted total is **2,300 SEK**. The difference is not A's posted balance.
- Record a 500 SEK transfer from B to A: A = **800 SEK**; B = **1,500 SEK**;
  A's planning difference is now **0 SEK**.
- Confirm the actual 800 SEK payment, then retry confirmation: A = **0 SEK**;
  B = **1,500 SEK**; spending = **800 SEK** once; income = **0 SEK**.
- That occurrence is no longer pending; the transfer was not spending.

## EX-09 — Forecast interest versus an actual credited payment

Savings account S starts at 1,000 SEK. Suppose a forecast displays 5 SEK of
interest; this is an illustrative supplied estimate, not a calculation formula.
The bank actually credits 4 SEK into S. Confirm that actual payment, then retry.

- Before confirmation: posted savings = **1,000 SEK**; realized income =
  **0 SEK**. Reaching the forecast payout date does not change these values.
- After confirmation and retry: S = 1,000 + 4 = **1,004 SEK**;
  interest income = **4 SEK**; spending = **0 SEK**.
- Exactly one actual interest transaction and one +4 SEK movement exist. Do not
  also post the 5 SEK forecast or book its 1 SEK difference as an expense.
- A later annual-rate change from 2% to 3% preserves both effective-dated periods
  and leaves the **1,004 SEK** posted balance and **4 SEK** income unchanged.
  Forecast calculation and capitalization conventions remain a P08 specification.

## EX-10 — Savings rejects ordinary income, including later assignment

Normal account A starts at 2,000 SEK and savings account S at 1,000 SEK.
Attempt to record 200 SEK of work income directly into S. After rejection,
record that income without an account, then attempt to assign it to S. Finally
assign it to A and transfer 200 SEK from A to S.

- Direct creation fails: A = **2,000 SEK**; S = **1,000 SEK**; income =
  **0 SEK**; no new transaction or movement is created.
- Recording without an account yields income = **200 SEK**, with both balances
  unchanged. The rejected savings assignment leaves that entry accountless,
  income at **200 SEK**, and both balances unchanged.
- Valid assignment to A: A = **2,200 SEK**; S = **1,000 SEK**.
- Transfer to S: A = **2,000 SEK**; S = **1,200 SEK**; overall total =
  **3,200 SEK**; income remains **200 SEK** and spending **0 SEK**.
- Savings receives a transfer, not work income relabeled as interest.

## Traceability and limits

| Examples | Product decision |
| --- | --- |
| EX-02–EX-04, EX-10 | [Accountless income and expenses — P00.05](accountless-entries.md) |
| EX-01, EX-05–EX-07 | [Currencies and transfers — P00.06](currencies-and-transfers.md) |
| EX-01, EX-05, EX-09–EX-10 | [Savings representation and accounting — P00.07](savings.md) |
| EX-08 | [US-02, US-03, and US-06 — P00.03](user-stories.md) |

Terms follow the [financial glossary](glossary.md). These ten examples complement,
rather than replace, the master plan's broader
[release acceptance cases](../plans/master-plan.md#8-acceptance-examples-required-before-public-release).
Authorization, unsupported inputs, interest formulas, and calendar policies still
need their later implementation specifications and automated tests.
