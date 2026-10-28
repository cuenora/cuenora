# Primary target user and problem

**Roadmap:** P00.02 · **Defined:** 2026-10-22

Cuenora is for people concerned about their personal finances who want control
over where their money goes, how much they spend, and how much remains for savings.

## Primary problem

Without a clear view of income, spending, and upcoming payments, people struggle
to understand their financial situation and decide how much they can set aside.
They need to connect everyday spending with its effect on the money remaining.

## Desired outcomes

- Understand where money goes through dated expenses and category breakdowns.
- See how much comes in and how much goes out over a selected period.
- Understand what remains and consider upcoming payments before deciding how
  much to save.

## Initial user evidence

The maintainer is the only current user. They have many bank accounts, move money
between them, and pay from different accounts. They currently open each bank to
check what remains. Sometimes a scheduled payment's account lacks funds because
they forgot to check or transfer money, even though another account has enough.

The initial focus is therefore visibility into each account and the payments
expected from it, alongside the broader spending and savings needs above. A
combined total alone cannot explain whether the payment account has enough money.

## Implications for the MVP

The [existing MVP scope](../plans/master-plan.md#1-release-scope) supports these
needs through accounts, income and expenses, recurring expenses, savings, and
basic analytics. This definition does not require users to own multiple accounts
or have financial expertise.

An account balance, a period's net income minus expenses, and money available to
save answer different questions. Future stories and screens must distinguish
them; a positive balance alone does not establish how much can be saved when
payments are still pending. P00.02 does not define a savings calculation or add
automatic savings advice to the MVP.

## Validation boundary

This is the maintainer-defined target audience and problem, not a claim of
external user research. The [prioritized stories](user-stories.md) (P00.03) build
on the maintainer's experience; the [PRD](prd.md) (P00.10) builds on both.
