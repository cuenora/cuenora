# Recurrence, Savings, and Insights wireframes

**Roadmap:** P01.08 · **Status:** Sketched initial proposal · **Date:** 2026-10-22

Recurrence separates expected payments from actual expenses. Savings separates
recorded balances, optional estimates, and credited interest. Insights explains
realized income and spending in each currency. These pen.dev sketches continue
[Home](home-wireframe.md) and [Accounts and Transactions](accounts-and-transactions-wireframes.md)
using their navigation, grayscale styling, and financial distinctions.

## Review the sketches

Open the [twelve-page PDF](wireframes/recurrence-savings-insights/recurrence-savings-insights-wireframes.pdf)
or the individual exports:

| Sketch | Scope | pen.dev frame ID |
| --- | --- | --- |
| [Recurrence](wireframes/recurrence-savings-insights/recurrence-overview.png) | Pending payments, selected date, accountless occurrence | `kTAYW` |
| [Add recurring expense](wireframes/recurrence-savings-insights/add-recurring-expense.png) | Amount, account, frequency, dates, explicit time zone | `oELfw` |
| [Occurrence detail](wireframes/recurrence-savings-insights/occurrence-detail.png) | Planning context and actual-payment entry | `TX2kv` |
| [Confirm payment](wireframes/recurrence-savings-insights/confirm-payment.png) | Actual amount/date/account and reviewed effects | `RvSNe` |
| [Savings](wireframes/recurrence-savings-insights/savings-overview.png) | Posted savings subset and transfer entry | `Z1CwQy` |
| [Savings detail](wireframes/recurrence-savings-insights/savings-detail.png) | Recorded balance, unavailable estimate, actual receipts | `xxExF` |
| [Interest terms](wireframes/recurrence-savings-insights/interest-terms.png) | Annual basis, effective date, payout and calculation inputs | `ARmOG` |
| [Interest receipt](wireframes/recurrence-savings-insights/record-interest.png) | Actual amount and independently selected receiving account | `SyJ8M` |
| [Insights](wireframes/recurrence-savings-insights/insights-overview.png) | Period/currency flow, category shares, distinct current balances | `dmBaV` |
| [Category detail](wireframes/recurrence-savings-insights/category-detail.png) | Contributing expense and accountless reporting | `SI8Ki` |
| [State specimens](wireframes/recurrence-savings-insights/state-specimens.png) | Empty, overdue, unsupported, failed, and confirmed states | `ChBXw` |
| [Interaction notes](wireframes/recurrence-savings-insights/interaction-notes.png) | Estimate specimen, rule actions, fixture stages, constraints | `mE9Q6` |

Editable layers are in the [pen.dev design document](../cuenora-design.pen), under
root names starting `P01.08`. Open the `.pen` file in pen.dev to edit the canvas;
PNG/PDF files are review snapshots. Earlier sketches are preserved. Phone frames show full
scroll content at 390 px width, not a fixed device height. These are static
wireframes, not an implemented application or a calculation engine.

## Recurrence

- Upcoming payments have explicit due dates, currencies, and source accounts.
  The EUR subscription remains visibly unassigned and outside every account's
  planning difference. The selected date is illustrative, not a default policy.
- An occurrence stays planned until an actual payment is confirmed. An overdue
  date alone has no accounting effect. Review is read-only; Not paid yet returns
  without posting. Confirmation reviews the actual amount, business date, and
  optional account before creating one expense and removing that occurrence from
  pending payments. Other occurrences remain separate.
- Add recurring expense shows a monthly draft on the 15th with explicitly selected
  UTC. Neither is a global default. Weekly, monthly, quarterly, and annual schedules
  remain planned capabilities. The sample does not choose a policy for days 29–31;
  the later [P07.01 calendar policy](recurrence-calendar.md) now defines it.
- Manage rules and View recurring rule lead to the rule-action specimen. Editing
  or pausing a rule requires review of affected occurrences; skipping or postponing
  applies to the selected occurrence without posting it. The later
  [P07.02 lifecycle specification](recurrence-lifecycle.md) defines transitions and
  preservation; implementation remains P07.03–P07.12, not behavior supplied by
  drawing these controls.

## Savings and interest

- Savings S is the same underlying account used in Accounts. Its SEK 1,000 is
  already included in the overall SEK total. New contributions use transfers;
  no ordinary-income shortcut is introduced. Interest configuration is optional.
- Missing terms show **No estimate available**, not zero interest. The terms draft
  distinguishes a 3% nominal annual rate from monthly payout frequency and retains
  an explicit effective date. Payout timing, day count, balance timing,
  capitalization, and rounding are deliberately unspecified in the draft.
- The interface must not interpret unidentified or effective rates as nominal.
  Estimates require complete, supported terms under P08.01. The notes include an
  independent **SEK 5 supplied estimate from EX-09**, solely to sketch the available
  state and its calculation-details entry. It was not computed from the draft rate;
  no interest formula or future return is asserted.
- Rate history preserves earlier effective periods; a new rate cannot rewrite
  actual credited receipts. Detailed history/calculation views remain implementation
  work. Account tracking and receipt recording remain available without a forecast.
- Record credited interest uses the actual received amount and date, then previews
  the effect in the selected receiving account. A receipt into another account is
  not also credited to Savings S. Cancellation changes nothing; confirmation and
  retries must record the receipt only once.

These distinctions follow [savings accounting](savings.md), the
[glossary](glossary.md), and [F05–F08](navigation-and-flows.md#primary-flows).

## Insights

The October example selects SEK explicitly; currency choices remain SEK, EUR, and
USD separately. Income, spending, and net flow use the selected business-date
period. Current recorded account balances and their savings subset are labeled
separately, not presented as period flow or money guaranteed available to save.

Category shares have text amounts and percentages alongside bars. Selecting a
category carries the period and currency into its contributing transactions, then
opens the existing record. Accountless expenses are included. Transfers, openings,
voided records, planned payments, and forecast interest are excluded from realized
income/spending. Category shares are omitted when spending is zero.

## Shared fixture and preview stages

The base snapshot is the P01.07 ledger: A SEK 2,500; B SEK 500; Savings S SEK 1,000;
Euro EUR 118; Dollar USD 100. It contains SEK 2,000 income, SEK 300 groceries, and
SEK 200 accountless transport spending. Planned payments are SEK 800 from A and an
independent EUR 20 unassigned subscription; neither affects realized totals.

| Check or independent preview | Expected result |
| --- | --- |
| Current balances and savings | SEK 4,000 overall, including SEK 1,000 savings; EUR 118 and USD 100 remain separate |
| October SEK report | Income 2,000; spending 500; net flow 1,500 |
| Spending categories | Groceries 300 = 60%; Transport 200 = 40%, including its accountless expense |
| Pending comparison through 30 October | A remains 2,500; pending 800; planning difference 1,700 |
| Later confirmation of the actual 800 payment on 30 October | A becomes 1,700; spending becomes 1,300 once; that occurrence leaves pending |
| Independent actual 4 interest receipt into A on 22 October | A becomes 2,504; Savings S remains 1,000; income becomes 2,004 once |
| Alternative: the same actual 4 receipt into Savings S | S becomes 1,004; A remains 2,500; do not apply both receipt variants |

The confirmation and receipt screens are independent, unsaved previews from the
base snapshot. The schedule and terms forms are separate drafts. The overdue and
SEK 5 estimate specimens are independent state examples, not additions to the
ledger. Later previews do not silently alter the main overview or report.

## Verification and remaining work

All twelve final frames were visually inspected; pen.dev's resolved traversal
reported no clipped nodes. Exact decimal checks verified totals, category shares,
the payment effect, and both independent receipt destinations. These checks cover
design fixtures, not implemented financial behavior. Repository validation, links,
PNG headers/dimensions, and the exported PDF are checked alongside this change.
Runtime tests do not apply to static sketches.

Loaded empty reports may show zero income/spending while account balances remain
nonzero. Loading and failed reads show unavailable data instead. An uncertain write
must resolve the original operation, never invite a duplicate payment or receipt;
exact controls follow the later API/client contract. Validation preserves inputs,
and cancellation preserves financial records.

P01.08 completes these sketches only. Calendar, interest, metric, filter, and retry
contracts still require their roadmap specifications. The subsequent
[palette](color-and-typography.md) and [design system](ui-design-system.md) complete
P01.09–P01.10 design scope; native implementation and accessibility tests remain pending.
