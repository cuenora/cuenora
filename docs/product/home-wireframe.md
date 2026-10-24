# Home wireframe

**Roadmap:** P01.06 · **Status:** Sketched initial proposal · **Date:** 2026-10-22

Home answers **“Where is my money, and which account needs attention?”**
This low-fidelity mobile sketch follows the [navigation proposal](navigation-and-flows.md)
and [verbal identity](verbal-identity.md). It is a design artifact, not an implemented
screen or an interactive prototype. Grayscale and Inter are temporary; P01.09 owns
palette and typography exploration.

## Review the sketch

Open the [complete five-page PDF](wireframes/home/home-wireframes.pdf), or inspect
the individual exports:

| View | Purpose | pen.dev frame ID |
| --- | --- | --- |
| [Recorded overview](wireframes/home/home-overview.png) | Currency totals, account-specific planning, upcoming payments, recording actions | `BF49A` |
| [First use](wireframes/home/home-first-use.png) | First account creation and accountless recording | `j0THl` |
| [Loading](wireframes/home/home-loading.png) | Unavailable values while reading data | `s1KS2` |
| [Load failure](wireframes/home/home-load-failure.png) | Explanation and retry without false zero balances | `IgJ2I` |
| [Design notes](wireframes/home/home-design-notes.png) | Interaction mapping, activity menu, and no-pending section copy | `g1AOk` |

The editable layers are in the [pen.dev design document](../cuenora-design.pen),
with root names beginning `P01.06 · Home`. Open the `.pen` file in pen.dev to edit
the canvas; these PNG and PDF exports are review snapshots. Preserve the frame
IDs in this table or update the mapping.

The populated screen shows the **entire scroll content** at a 390 px design width.
The other screens use a 390 × 844 px viewport. In implementation, system chrome
and navigation remain outside the scrolling body; the long export is not a device
height specification. Native presentation and larger text still need validation.

## Hierarchy and interactions

| Area | Content and intended action |
| --- | --- |
| Header | Home, a labeled-for-accessibility record-activity control, and Settings/session access. The `+` opens income, expense, transfer, and cancel choices. |
| Recorded balances | Separate SEK, EUR, and USD totals; no converted grand total. Savings, when present, is already included in its currency's total. Links open Accounts and Savings. |
| Planning date | Explicit selected date: 30 October 2026. Opens a date picker; return preserves the selection. This example does not establish a default date. |
| Account planning | Account A's recorded balance, pending payments, formula, and planning difference. Review opens its pending payments. Account B remains visibly distinct; other accounts are reachable through Accounts. |
| Upcoming payments | Dated planned occurrences with amount, currency, and source account. Review opens occurrence details; an unassigned occurrence stays visible and can remain unassigned. View recurrence opens the broader list. |
| Recording actions | Income/expense enter F02; transfer enters F04. Transfers record activity performed elsewhere. Fewer than two suitable accounts leads to account creation or cancellation. |
| Primary navigation | Four provisional destinations: Home, Accounts, Transactions, Insights. Recurrence and Savings have explicit Home links. Details return to the originating context. |

Payment confirmation belongs in occurrence details, after the payment happened.
Opening a row, reaching a due date, or reviewing a shortfall never posts an expense.
Cancellation, validation, unknown write outcomes, and session recovery follow
[P01.05's shared paths](navigation-and-flows.md#shared-alternate-and-recovery-paths);
this sketch does not introduce offline writes or new recovery contracts.

## Example data and financial checks

All amounts are synthetic. The SEK scenario reproduces
[EX-08](worked-examples.md#ex-08--a-scheduled-payment-needs-funds-in-its-source-account):

- Account A: SEK 300.00 recorded; SEK 800.00 pending on 30 October 2026;
  planning difference **−SEK 500.00**.
- Account B: SEK 2,000.00 recorded; no pending payments in this example.
  The SEK 2,300.00 combined balance does not eliminate A's shortfall.
- Recording an actual SEK 500.00 transfer B → A changes A to SEK 800.00 and B
  to SEK 1,500.00. A's planning difference becomes zero; spending stays zero.
- Confirming the actual SEK 800.00 payment leaves A at zero and records spending
  once. The occurrence leaves pending; retrying must not duplicate the expense.

Two independent illustrative accounts add EUR 400.00 and USD 120.00 to demonstrate
separate totals. A EUR 20.00 subscription due 28 October 2026 has **No account
assigned**. It appears in upcoming payments but is outside every account's planning
difference. These extra fixtures do not alter the EX-08 SEK scenario. Forecast
interest is absent from recorded balances. See the [currency rules](currencies-and-transfers.md)
and [PRD](prd.md) for the underlying product constraints.

## State and accessibility notes

- First use means no accounts or recorded activity. Add account is primary, while
  income and expense recording remain available without one. If accountless data
  already exists, preserve access through Transactions and Insights; do not imply
  that those records are missing or require an account.
- No pending payments is a section result, not an empty financial overview:
  retain balances, show the selected date, and offer another date or recurrence.
- Loading and load failure show unavailable data without fabricated zeros.
  Retry reloads data; it does not submit a financial operation.
- Shortfalls use a label, signed amount, formula, and icon, not color alone.
  Main actions have 48 px design hit areas; navigation items are 56 px high.
  Native accessibility labels, reading order, text scaling, and device-specific
  target sizes require implementation checks; this sketch is not an accessibility
  certification.

## Verification and follow-up

Canvas screenshots were visually inspected. A subsequent resolved-layout traversal
reported no clipped nodes across all five frames. The EX-08 amounts and accountless
paths were checked against the source documents; exports were generated through
the pen.dev MCP. Repository validation and whitespace checks passed.

No runtime tests apply to static wireframes. P01.07–P01.08 cover the other screens;
P01.09–P01.10 cover visual styling, reusable components, and the broader state
system. Planning defaults and precise period boundaries remain P09.01 decisions.
This task completes the Home sketch, not Phase 01 or mobile implementation.
