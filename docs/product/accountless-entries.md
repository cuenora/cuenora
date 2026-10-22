# Income and expenses without an account

**Roadmap:** P00.05 · **Status:** Accepted · **Date:** 2026-10-22

Income and expenses may be recorded without an assigned account. They contribute
to income and spending reports, but do not change any account balance. The
maintainer confirmed this behavior for both types of entry.

## Rules

- An account is optional; amount, currency, and business date still describe the
  entry. Accountless entries must be clearly identifiable in the interface.
- Reports include each entry once in its currency and business-date period,
  regardless of whether it has an assigned account.
- An accountless entry creates no account movement and does not change individual
  or combined account balances. It is not an implicit cash account.
- Assigning an account later creates the corresponding balance effect exactly
  once: income increases the balance and an expense decreases it. The original
  entry remains the same report item, with its original business date.
- Retrying the assignment must not duplicate the entry or its balance effect.
- Ordinary income cannot be assigned to a savings account, including through
  edits or later reassignment. Reject such an assignment without changing the
  entry, its report effect, or any balance; see the
  [savings funding rules](savings.md#funding-and-allowed-income).
- Removing or changing an account assignment must reverse the previous account
  effect and, where applicable, apply the new one atomically, preserving the
  auditable history required by the master plan.
- Account ownership and currency compatibility must be validated before linking;
  an invalid assignment leaves the entry and balances unchanged. Currency policy
  is recorded separately in P00.06.

## Worked acceptance examples

Each scenario starts independently with account A at 1,000 SEK and account B at
500 SEK. Entries and balances below are illustrative, not real financial data.

| Scenario | Expected account balances | Expected report effect |
| --- | --- | --- |
| Record a 200 SEK expense without an account. | A: 1,000 SEK; B: 500 SEK. | Spending increases by 200 SEK. |
| Record a 300 SEK income without an account. | A: 1,000 SEK; B: 500 SEK. | Income increases by 300 SEK. |
| Record the accountless 200 SEK expense, then assign it to A and retry that assignment. | A: 800 SEK; B: 500 SEK. | One 200 SEK expense, in the original business-date period. |
| Record the accountless 300 SEK income, then assign it to A and retry that assignment. | A: 1,300 SEK; B: 500 SEK. | One 300 SEK income, in the original business-date period. |
| Record a 200 SEK expense assigned to A, then remove its assignment. | A: 1,000 SEK; B: 500 SEK. | One 200 SEK expense remains, now without an account. |
| Record a 200 SEK expense assigned to A, then change its assignment to B. | A: 1,000 SEK; B: 300 SEK. | One 200 SEK expense remains. |

## Consequence and implementation boundary

This supports recording an entry before identifying its account. Until entries
are assigned, recorded balances may differ from actual bank balances; reports
and account totals answer different questions. Accountless entries must not be
silently allocated to an account to make those totals match.

This document records product behavior, not implemented functionality or executed
financial tests. Persistence, API behavior, UI, and automated acceptance tests
belong to later roadmap tasks. See the
[financial rules](../plans/master-plan.md#accounts-and-accountless-entries) and
[prioritized stories](user-stories.md).
