# Financial glossary

**Roadmap:** P00.08 · **Status:** Defined · **Date:** 2026-10-22

These terms describe Cuenora's agreed product behavior. They do not imply that
financial functionality is already implemented.

## Accounts and recorded money

| Term | Meaning in Cuenora |
| --- | --- |
| Account | A user-owned record of money held in one currency. Its balance derives from posted movements, not a client-supplied balance. It does not imply a connection to a bank. |
| Normal account | An account that can receive ordinary income, such as salary or payment for work. |
| Savings account | An account designated for savings, with or without interest. It allows an opening balance at creation; subsequent contributions enter through transfers, and its only direct income is actual credited interest. |
| Savings | The posted balances of designated savings accounts, grouped by currency. These are a subset of account balances, not additional money to add to overall totals. |
| Opening balance | Money already held when an account is created, recorded as a special opening movement. It is neither income nor an invented transfer. |
| Balance | The sum of an account's posted signed movements in its currency. It excludes future payments and forecast interest and reflects recorded activity, not necessarily the current bank balance or money safe to spend. |
| Currency | The monetary unit attached to an amount and account. The MVP supports SEK, EUR, and USD; different currencies remain separate in totals. |

## Operations and their effects

| Term | Meaning in Cuenora |
| --- | --- |
| Transaction | A recorded business operation, such as income, an expense, or a transfer. It describes what happened; account movements describe its balance effects. One transfer is one transaction even though it affects two accounts. |
| Movement | A signed posting to one account in that account's currency. A positive movement increases its balance and a negative movement decreases it. It is distinct from the business operation that caused it. |
| Posted | Recorded as an actual event with its applicable account effects. Posting an accountless income or expense affects reports but creates no account movement. |
| Income | Money received and counted in income reports. Ordinary income may be accountless or assigned to a normal account; it cannot be assigned to savings. Credited interest is a distinct income type. Transfers and opening balances are not income. |
| Expense | Spending counted in expense reports. An assigned expense decreases its account balance; an accountless expense affects reports without changing a balance. Transfers are not expenses. |
| Accountless entry | An income or expense with no assigned account. It appears in reports but creates no account movement. Later valid assignment applies the account effect exactly once without duplicating the report entry. |
| Transfer | One operation moving recorded money between two distinct accounts owned by the same user. It posts a source debit and destination credit atomically and does not count as income or spending. Cuenora records it rather than executing a bank payment. |
| Exchange rate | An optional transfer reference quoted as destination currency units per one source currency unit. Cross-currency transfers require the sent and received amounts; those confirmed amounts determine postings. The rate does not authorize conversion of overall totals. |
| Net flow | Realized income minus expenses for a business-date period, separately by currency. It is not an account balance or a guarantee of money available to save. |
| Idempotency | Retrying the same write operation does not create a second transaction or repeat its balance effects. |

For example, recording a 200 SEK expense without an account creates one report
item and zero movements. Linking it to a normal account later creates a −200 SEK
movement and keeps one report item. A transfer sending 200 SEK and receiving
18 EUR instead creates one operation with two movements, −200 SEK and +18 EUR;
those amounts cannot be added across currencies.

## Dates, plans, and interest

| Term | Meaning in Cuenora |
| --- | --- |
| Business date | The date the user assigns to the economic event, distinct from when the system records or updates it. Reports use this event date. |
| Audit timestamp | A UTC timestamp recording when the system created or changed a record. It does not replace the business date. |
| Due date | The date a scheduled payment or occurrence is expected. Reaching it does not itself confirm payment or change a balance. |
| Recurring rule | A schedule defining expected repeated expenses and their terms. It generates planned occurrences, not automatically posted spending. |
| Occurrence | One scheduled instance of a recurring expense, with its own due date and state. Confirming it posts the actual expense exactly once. |
| Planned | Expected future activity that has not been confirmed or posted. It does not change balances or realized income and expenses. |
| Forecast interest | Estimated interest under explicit calculation terms. It remains separate from posted balances and realized income, even when its expected payment date arrives. |
| Credited interest | An actual interest payment confirmed as received. It creates interest income and a matching movement in the receiving account exactly once; it is not also credited to a different account. |
| Annual rate | An interest percentage quoted on an annual basis, independent of payment frequency. A nominal annual rate must be identified explicitly; an annual percentage alone does not establish whether it is nominal or effective. |
| Rate period | The effective-dated interval during which a rate applies. Changing a rate preserves previous periods and does not rewrite actual credited payments. |
| Payout frequency | How often interest is paid: daily, monthly, quarterly, or annually. It is distinct from the rate quotation and the calculation's day-count convention. |
| Capitalization | Adding credited interest to the interest-bearing principal according to explicit terms. It is distinct from contributing money by transfer; a forecast never capitalizes itself into the posted balance. |

## Decision references

- [Accountless income and expenses — P00.05](accountless-entries.md)
- [Currencies and transfers — P00.06](currencies-and-transfers.md)
- [Savings representation and accounting — P00.07](savings.md)

Calculation precision, rounding, and interest conventions still require the
implementation specifications identified in those decisions. This glossary does
not introduce a bank-specific formula or additional financial features.
