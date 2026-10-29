# Accounts and Transactions wireframes

**Roadmap:** P01.07 · **Status:** Sketched initial proposal · **Date:** 2026-10-22

Accounts explains **where recorded money is held**; Transactions explains **what
happened and when**. These low-fidelity pen.dev sketches extend the
[Home wireframe](home-wireframe.md) and [primary flows](navigation-and-flows.md).
They are static design artifacts, not working screens or approved API contracts.

## Review the sketches

Open the [ten-page PDF](wireframes/accounts-transactions/accounts-transactions-wireframes.pdf)
or an individual PNG:

| Sketch | Scope | pen.dev frame ID |
| --- | --- | --- |
| [Accounts](wireframes/accounts-transactions/accounts-overview.png) | Normal and savings accounts, currency groups, creation | `PDaOP` |
| [Account detail](wireframes/accounts-transactions/account-detail.png) | Posted activity and separate pending-payment comparison | `InPc2` |
| [Add account](wireframes/accounts-transactions/add-account.png) | Name, type, currency, opening balance and date | `a6mwI` |
| [Transactions](wireframes/accounts-transactions/transactions-overview.png) | Dated operations, accountless expense, single transfer entry | `EBFeU` |
| [Filters](wireframes/accounts-transactions/transaction-filters.png) | Dates, currency, account, unassigned entries, type, category | `XtxVR` |
| [Record expense](wireframes/accounts-transactions/record-expense.png) | Dated expense with optional account assignment | `UZrhR` |
| [Expense detail](wireframes/accounts-transactions/expense-detail.png) | Assignment, editing, and audit-history access | `v3Azl` |
| [Record transfer](wireframes/accounts-transactions/record-transfer.png) | Explicit sent/received amounts and optional reference rate | `bD37z` |
| [State specimens](wireframes/accounts-transactions/state-specimens.png) | Empty data, no matches, failed reads, invalid input, uncertain writes, cancellation | `ORpbu` |
| [Interaction notes](wireframes/accounts-transactions/interaction-notes.png) | Assignment review, fixture ledger, and financial constraints | `bb4MH` |

Editable layers remain in the same pen.dev document as Home, under root names
starting `P01.07`. The source is the [editable design document](../cuenora-design.pen).
Open it in pen.dev to edit the canvas; PNG/PDF files are review snapshots. Home's
existing layers and exports are preserved.

## Screen and interaction decisions

- **Accounts:** Group accounts by currency and show each recorded balance. Savings
  is an account type, already included in its currency total. Opening it through
  Accounts or Savings reaches the same account, not duplicated money.
- **Account detail:** Separate posted activity from planned occurrences through
  the selected date. The planning difference is labeled and never replaces the
  balance. Opening movements appear in account history but are not income.
  Income, expense, and transfer actions retain the source account context.
- **Account creation:** Choose Normal or Savings and a supported currency. An
  opening balance records existing money. Review precedes confirmation; interest
  setup is not required to create a savings account.
- **Transactions:** Show each recorded operation once, ordered by business date
  in this proposal. A cross-currency transfer displays both actual amounts in one
  entry; neither leg is income or spending. Planned payments remain in Recurrence.
- **Filters:** The October date range is illustrative, not a new default policy.
  Account choices include all accounts, a specific account, and no account assigned.
  Applying uses the draft selection; cancelling retains the applied filters.
  Reset clears user restrictions with the resulting range visible. Exact filter
  defaults and transfer/category matching remain implementation specifications.
- **Income and expense:** The expense sketch demonstrates the shared amount,
  currency, business-date, description, category, and account fields. Income is
  selected through Record activity and uses the same structure with income copy.
  An assigned account must match the currency. Ordinary income permits only a
  normal account or no account, including later edits and reassignment.
- **Record detail:** Assign or correct the existing operation and preserve its
  history. The assignment review previews the account effect while explaining
  that the business date and report item remain unchanged. No routine deletion
  of posted movements is introduced.
- **Transfers:** Choose distinct owned accounts. Same-currency amounts match;
  different currencies require sent and received amounts. The optional rate is
  quoted as destination units per source unit and cannot overwrite final amounts.
  Review repeats both movements before confirmation. No bank payment is executed.

Forms lead to review before confirmed persistence. Success returns to the
originating context; cancellation changes no records. The review specimen shows
assignment in detail; other confirmation screens use the same review-before-save
principle without claiming that all later form states have been designed.
Native presentation, larger text, and accessible names remain implementation work.
The phone frames show full scroll content at 390 px width, not fixed device heights.
Main buttons use 48 px targets; navigation and headers reuse Home's conventions.

## Synthetic ledger and walkthrough

This set uses one independent fixture, rather than silently modifying Home's
EX-08 example. It combines the distinctions in
[EX-01–EX-08 and EX-10](worked-examples.md); the source examples themselves remain
independent acceptance cases.

| Recorded operation | Account effect |
| --- | --- |
| 1 October: openings | A SEK 1,000; B SEK 500; Savings S SEK 1,000; Euro EUR 100; Dollar USD 100 |
| 18 October: work income, SEK 2,000 | A becomes SEK 3,000 |
| 19 October: groceries, SEK 300 | A becomes SEK 2,700 |
| 20 October: transfer, SEK 200 sent / EUR 18 received | A becomes SEK 2,500; Euro becomes EUR 118 |
| 21 October: transport expense, SEK 200, no account | No account balance changes; the expense appears in reports |

Resulting recorded totals are **SEK 4,000**, **EUR 118**, and **USD 100**, separately.
Savings S's SEK 1,000 is already within the SEK total. Recorded SEK income is
2,000 and spending is 500; transfers and openings are excluded.

A planned SEK 800 payment from A on 30 October leaves A's balance at SEK 2,500
and gives a **SEK 1,700 planning difference**. It is not recorded spending yet.
The later assignment-review specimen would reduce A from SEK 2,500 to SEK 2,300
for the existing transport expense. Spending stays SEK 500, and its original
21 October business date stays intact. The review is not an already-applied change.

The expense and transfer forms show the existing sample operations **before**
confirmation; they are not additional operations to apply to this ledger. The
Add account form is an independent unsaved draft, excluded from all totals.

## Recovery and verification

Empty Accounts preserves accountless recording. Empty Transactions and filtered
no-results have different explanations and actions. Read errors show unavailable
data, never zero balances. Loading follows the existing Home pattern. Validation
retains input and changes nothing; an unknown write result must resolve the
original operation rather than create another. Exact retry/status controls remain
owned by the later API/client contract. See [shared recovery paths](navigation-and-flows.md#shared-alternate-and-recovery-paths).

All ten final frames were visually inspected and checked through pen.dev's resolved
layout traversal: no clipped nodes were reported. Exact decimal checks verified
the ledger totals, planning difference, and assignment preview. These are checks
of the design fixtures, not executed financial-feature tests. Repository, link,
and exported-file checks accompany this documentation change; no runtime test
suite applies to static sketches.

P01.07 completes these sketches only. [Recurrence, Savings and Insights sketches](recurrence-savings-insights-wireframes.md)
and the [visual/component system](ui-design-system.md) are now documented.
Use the implemented [money](money.md) and [account](accounts.md) policies; calendar,
filtering and correction use-case contracts still need their implementation specifications.
