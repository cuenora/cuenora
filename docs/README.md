# Documentation map

Start with the user's problem, check what exists, then choose one deliverable.
Cuenora aims to make manually recorded balances and upcoming payments visible
**per account**, without implying bank synchronization, payment execution or
guaranteed spending capacity.

## From today's foundations to the MVP

1. Read the [PRD](product/prd.md) and [prioritized stories](product/user-stories.md)
   for outcomes and release boundaries, not implementation claims.
2. Check the [delivery snapshot](plans/master-plan.md#current-delivery-snapshot)
   and [dependency order](plans/master-plan.md#13-current-implementation-order).
   Money/accounts, storage and authentication foundations do not yet form a
   usable financial journey; mobile clients are not initialized.
3. Select one focused task from the [backlog mapping](product/backlog.md).
   For the first account API slice, include verified-email and ownership gates;
   dated openings/transactions additionally need confirmed-zone validation.
4. Read the relevant approved policy, ADR and current code/tests before changing
   behavior. Use [setup](development/SETUP.md) for runnable checks and
   [planning guidance](plans/README.md) when an execution plan is warranted.
5. Deliver behavior, tests and documentation together. Then build transfers,
   confirmed recurring payments and per-account funding comparisons; the latter
   need not wait for interest forecasts. Follow the master plan for mobile and
   release-readiness dependencies rather than assuming a whole phase is complete.

## Which document answers which question?

| Question | Primary reference |
| --- | --- |
| Why this product; what is inside/outside MVP? | [PRD](product/prd.md), [target user](product/target-user.md) |
| What is complete and what comes next? | [Master plan](plans/master-plan.md); linked execution records provide evidence |
| What do financial terms and examples mean? | [Glossary](product/glossary.md), [worked examples](product/worked-examples.md) |
| What are the approved domain rules? | [Money](product/money.md), [accounts](product/accounts.md), [dates](product/dates-and-time-zones.md), [accountless entries](product/accountless-entries.md), [transfers](product/currencies-and-transfers.md), [savings](product/savings.md) |
| What protects identity and financial access? | [Identity strategy](product/identity-and-access.md), [verification/recovery design](product/email-verification-and-recovery.md), [security policy](../SECURITY.md) |
| Why were technical choices made? | [ADR index](architecture/decisions/README.md); accepted decisions and original tradeoffs |
| What is stored, and what must use cases still enforce? | [Identity](architecture/identity-storage.md), [accounts](architecture/account-storage.md), [categories](architecture/category-storage.md), [transactions](architecture/transaction-storage.md), [movements](architecture/account-movement-storage.md), [audit](architecture/transaction-audit-storage.md), [constraints](architecture/financial-constraints.md) |
| Which HTTP operations actually exist? | [OpenAPI](../contracts/openapi.yaml), implemented routes and tests; illustrative roadmap paths are not shipped endpoints |
| How should native screens behave and look? | [Design guide](../DESIGN.md), [navigation](product/navigation-and-flows.md), [UI system](product/ui-design-system.md); static designs are not working clients |
| How do I build, test and contribute? | [Setup](development/SETUP.md), [contributing](../CONTRIBUTING.md), [agent instructions](../AGENTS.md) |

## Keep the path trustworthy

- Keep current completion in the master plan rather than duplicating task
  checkboxes across product, UX and architecture guides.
- Product specifications define required behavior; code and executed tests show
  what exists. Investigate disagreements instead of assuming either is correct.
- ADRs and completed execution plans preserve decisions and evidence at their
  dates. Add a dated status note or superseding ADR when appropriate; do not
  rewrite history to make it look as if later features existed earlier.
- State whether a deliverable is design, domain, storage, API or client behavior.
  Local verification, publication, merge and release readiness are different facts.
- Update the contract/help/docs affected by behavior changes, retain task IDs,
  and report failed, ignored or unavailable checks. Unresolved recurrence,
  interest, archive and API policies remain decisions to make, not defaults.

Account creation is implemented locally with own/independent checks and native review passed; not yet published; see the
[contract](product/account-creation.md) and [active execution plan](plans/completed/2026-10-29-account-creation.md).
It requires verified email and lifetime idempotency. Email verification runtime
remains pending, so this does not establish a usable financial journey.

P05.03 adds [dated opening balances and explicit profile zones](product/opening-balances.md)
locally; own/independent checks and native review passed; publication pending. This does not implement email verification
or a complete profile/financial journey.

P05.04–P05.05 add merged [verified-owner account listing and exact balances](product/account-listing.md).
P05.06 adds [renaming, zero-balance archive and reactivation](product/account-management.md);
own/independent proof passed locally and native review acknowledged; publication pending.
