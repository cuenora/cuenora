# Prioritized user stories

**Roadmap:** P00.03 · **Status:** Defined · **Date:** 2026-10-22

These ten stories prioritize the [initial user's problem](target-user.md): money
is spread across bank accounts, and a scheduled payment can lack funds in its
source account even when money exists elsewhere. Priority reflects user value,
not implementation order. These specify target financial journeys; implemented
domain/storage and authentication foundations do not yet complete those journeys.
Use the [master-plan snapshot](../plans/master-plan.md#current-delivery-snapshot) for current progress.

## Stories

| Priority / ID | User story | Observable outcome |
| --- | --- | --- |
| 1 / US-01 | As a person with several accounts, I want to see each account's recorded balance in one place so I can understand where my money is. | Each account shows its currency and balance derived from posted movements; totals remain separate by currency. These are recorded balances, not live bank balances. |
| 2 / US-02 | As someone paying from different accounts, I want to associate recurring payments with their source account and due date so I know which account needs money and when. | Upcoming occurrences show amount, currency, due date, and assigned account; planned payments do not reduce posted balances. Unassigned occurrences remain visibly unassigned. |
| 3 / US-03 | As someone preparing for upcoming payments, I want to compare each account's recorded balance with its pending payments over a selected period so I can identify a possible shortfall. | For each account, show the recorded balance, pending scheduled expenses through the selected date, and their difference. Label this a planning estimate; it excludes unrecorded activity and does not change the balance. |
| 4 / US-04 | As someone moving money between banks, I want to record transfers between my accounts so my records reflect where the money went. | A transfer changes both balances together exactly once and does not appear as income or spending. Same-currency amounts are equal; cross-currency transfers record sent and received amounts with an optional rate (see [P00.06](currencies-and-transfers.md)). Recording it does not instruct a bank to move money. |
| 5 / US-05 | As someone receiving income and paying from different accounts, I want to record dated income and expenses against the relevant account so its recorded balance stays useful. | A posted entry affects only its assigned account and appears in the appropriate dated income or expense report. Ordinary income may target normal accounts, not savings, including on later reassignment; savings accepts only credited interest as direct income. Opening balances remain distinct from income. |
| 6 / US-06 | As someone tracking recurring payments, I want to confirm a payment once it happens so I can distinguish paid expenses from upcoming commitments. | Confirmation posts the expense exactly once and removes that occurrence from pending commitments; future occurrences remain planned. |
| 7 / US-07 | As someone concerned about spending, I want to see how much I spend by category and period so I can understand where my money goes. | Reports include dated expenses with or without an account and exclude transfers and opening balances. Currency totals remain separate. |
| 8 / US-08 | As someone deciding how much to save, I want to see my income, spending, and upcoming commitments separately so I can make an informed savings decision. | The view distinguishes realized net flow, recorded balances, and pending commitments; it does not label their combination as guaranteed safe-to-save money. |
| 9 / US-09 | As someone who cannot yet identify which account paid an expense, I want to record it without an account so I do not lose track of my spending. | The expense appears in reports without changing an account balance; assigning an account later affects that balance exactly once. |
| 10 / US-10 | As someone holding interest-bearing savings, I want to distinguish projected interest from payments actually received so I do not plan with money I have not received. | Forecast interest remains separate from balances and realized income; a confirmed credited payment affects the savings account and income exactly once. Interest calculation policies require definition before implementation. |

## Scope and decisions

The existing [release scope](../plans/master-plan.md#1-release-scope) excludes bank
connections from the MVP and places reminders in v1. These stories therefore
assume records maintained by the user, not automatic bank synchronization.
US-03 proposes an in-app planning comparison within basic analytics; its exact
presentation and calculation rules still need specification before implementation.

The maintainer confirmed on 2026-10-22 that reminders are not essential for the
first version and can follow later, consistent with their existing v1 placement.
The MVP supports checking balances and upcoming payments inside the app. It does
not proactively notify the user to check or fund an account.

Exact money and authentication foundations exist. Financial ownership checks,
verified-email authorization and the remaining financial workflows still need
runtime implementation and proof; all master-plan invariants apply to each story.
