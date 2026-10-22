# Cuenora verbal identity

**Roadmap:** P01.04 · **Status:** Defined · **Date:** 2026-10-22

Cuenora speaks clearly, warmly, and precisely. It helps people understand their
recorded money and upcoming payments without judging their choices or sounding
like a bank. This guide applies to product copy and public product descriptions.
Examples describe intended experiences, not features already implemented.

## Core message

**Open-source personal finance, clearly organized.**

Supporting description: “Keep track of your accounts, spending, and upcoming
payments in one place.”

The voice serves the [primary target user](target-user.md): someone who wants
to understand where money goes and which account needs funding. Trust comes
from accurate explanations and visible limits, not institutional language or
promises about financial outcomes.

## Voice principles

| Principle | Write this way | Avoid |
| --- | --- | --- |
| Clear | Use familiar words, short sentences, and concrete amounts with currencies. | Jargon, vague warnings, and unexplained accounting terms. |
| Approachable | Address the person directly and explain the next useful action. | Blame, moral judgments about spending, forced enthusiasm, and pressure. |
| Trustworthy | Say what was recorded, what is planned, and what remains uncertain. | Guaranteed savings, returns, absolute security, or claims that records are live bank data. |
| Non-banking | Describe organizing and recording money. | Implying Cuenora holds funds, connects to banks, or executes payments in the MVP. |

Use neutral, everyday English. Prefer sentence case for headings and actions.
Avoid ALL CAPS, alarmist punctuation, and celebratory messages about spending
less or earning more. A missed payment date is a state to explain, not a personal
failure. Keep amounts, currencies, and dates explicit where they affect meaning;
do not rely on color alone to communicate a shortfall or error.

## Tone by situation

| Situation | Tone and purpose | Example |
| --- | --- | --- |
| Welcome | Warm and practical; explain the first step. | “Welcome to Cuenora. Add an account to start organizing your recorded balances.” |
| Empty accounts | Reassuring; explain why the screen is empty. | “No accounts yet. Add an account to track its recorded balance.” |
| Empty period | Factual; scope the absence to the current view. | “No expenses recorded for this period.” |
| Successful record | Brief; confirm the action that actually completed. | “Expense recorded.” |
| Validation | Specific and actionable; preserve the person's dignity. | “Choose different source and destination accounts.” |
| Loading | Calm; describe the work in progress. | “Loading your accounts…” |
| Read failure | Honest; offer an available recovery action. | “We couldn’t load your accounts. Try again.” |
| Funding shortfall | Precise; identify the account, period, and recorded-data basis. | “Everyday account: upcoming payments through 31 October 2026 exceed its recorded balance by SEK 200.” |

Only offer actions that the interface supports. For a write whose outcome is
unknown, do not claim either success or failure, unchanged balances, or a safe
retry unless the implementation can establish it. Example: “We couldn’t confirm
whether the expense was recorded.” Recovery wording must match the implemented
idempotency and status-check behavior.

## Financial wording

Use the [financial glossary](glossary.md) as the semantic authority. Plain wording
must preserve its distinctions; it must not introduce a new financial rule.

| Concept | Preferred wording or example | Avoid |
| --- | --- | --- |
| Balance | “Recorded balance: SEK 300.” | “Available to spend: SEK 300” when only posted movements are known. |
| Pending payments | “Pending payments through 31 October 2026: SEK 800.” | Suggesting a due date automatically posts an expense or pays a bill. |
| Planning difference | “Recorded balance minus pending payments: −SEK 500.” | Calling the result the account balance or guaranteed money available to save. |
| Transfer | Action: “Record transfer.” Confirmation: “Transfer recorded.” | “Send money” or “Payment sent”; Cuenora records an operation performed elsewhere. |
| Accountless entry | Label: “No account assigned.” Explanation: “Included in spending reports; no account balance changed.” | Suggesting the expense is missing from reports or automatically assigning an account. |
| Opening balance | “Opening balance.” | Calling existing money income. |
| Forecast interest | “Estimated interest”; explain “Not yet credited. Excluded from recorded balances.” | Presenting a projection as earned income, credited money, or a guaranteed return. |
| Actual interest | “Credited interest” for an actual payment confirmed as received. | Implying the forecast or arrival of a payout date confirmed receipt. |
| Multiple currencies | Show SEK, EUR, and USD totals separately. | An unlabeled combined total or implied automatic conversion. |

A funding comparison needs a selected date and the relevant source account.
An unassigned upcoming payment must remain visibly unassigned. A combined balance
must not imply that the account used for a payment has enough money. Detailed
metric boundaries remain owned by P09.01; this guide does not define them.

Use “record,” “add,” “review,” and “confirm” only when they accurately describe
the action. For a planned occurrence, “Confirm payment” means confirming a payment
already made, not instructing Cuenora to pay it. Supporting copy should make that
distinction explicit: “Record this payment after you have paid it.”

## Copy comparisons

| Avoid | Prefer | Reason |
| --- | --- | --- |
| “You overspent again!” | “Expenses recorded this month: SEK 4,200.” | Describe recorded activity without judging it. |
| “Your finances are in danger.” | “Everyday account needs SEK 200 more to cover recorded pending payments through 31 October 2026.” | Name the concrete planning gap without predicting a bank outcome. |
| “You can safely save SEK 500.” | “Income minus expenses this month: SEK 500.” | Net flow does not establish money available to save. |
| “Your money is completely safe with us.” | “Cuenora records your finances; it does not hold your money.” | Explain the product boundary without an absolute security promise. |
| “Guaranteed earnings next month.” | “Estimated interest for November 2026.” | Keep projections distinct from actual payments and guarantees. |

## Review checklist

Before accepting copy, check that it:

- Describes the person's situation without blame or pressure.
- States the actual action or state and offers only supported next steps.
- Distinguishes recorded, planned, and estimated amounts with explicit currencies
  and relevant dates.
- Preserves accountless, transfer, and interest semantics from the glossary.
- Makes no unsupported claim about bank access, payment execution, security,
  returns, or money available to save.

P01.04 defines the verbal identity. User flows, wireframes, visual identity,
components, and implementation remain separate roadmap tasks. The examples
are a writing reference, not a final screen inventory or localization policy.
