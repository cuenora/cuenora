# Financial storage constraints

P03.10 adds `0008_financial_constraints.sql` without rewriting prior migrations
or financial records. It validates currency/account compatibility and prevents
ordinary income from being assigned to a savings account.

## Database guarantees

| Rule | Enforcement |
| --- | --- |
| Account type, supported currencies, operation kinds and amount bounds | Existing checks from migrations 0003, 0005 and 0006 remain unchanged. |
| Assigned transaction currency | Composite FK from account/owner/currency to the account. |
| Transfer destination currency | Composite FK from destination/owner/received currency. |
| Posting currency | Composite FK from movement account/owner/currency. |
| Ordinary income | Generated `income_account_type` requires `normal` for an assigned `income`; composite FK enforces account type. |
| User/date lookup | New transactions `(user_id, occurred_on)` index; existing posting indexes retained. |

Accountless income and expenses remain valid. Credited interest, opening balances,
expenses and transfers are not ordinary income; existing shape checks still apply.
Both creation and edits—including later account assignment—use the same guards.
The technical generated column is not a new editable field or audit-summary name.

Native immediate FKs reference unique account owner/currency and owner/type
keys. They protect both child writes and parent key changes under concurrency;
no application precheck or unsynchronized cross-table trigger is substituted.
A type/currency change is allowed only if existing references remain valid.
Linked ordinary income prevents normal-to-savings reclassification even when
voided. Accounts without incompatible references can change type or currency.
[ADR-008](decisions/008-account-reference-compatibility.md) records this policy.

## History and application boundary

Posting amount, sign, date and currency remain immutable snapshots. They are not
forced to equal the current event, nor is event/account membership or the number
of postings validated here. Atomic posting/correction sets, reversal links,
transfer idempotency, authenticated ownership and automatic audit recording remain
future application responsibilities (P05.13 and the transfer phase).

## Apply, verify and recover

Apply each migration in one transaction as documented in
[identity storage](identity-storage.md#apply-verify-and-recover). All new FKs
validate existing rows immediately. Incompatible legacy rows must abort the whole
migration; no `NOT VALID` escape, deletion, relabeling or automatic conversion is
provided. Investigate rejected legacy data before attempting a controlled repair.
A failed migration must preserve prior data and schema, including generated-column
and unique-target creation. Do not drop populated financial data for rollback.

Run `python3 tests/infrastructure/test_postgres_compose.py` with local Docker.
Tests use synthetic disposable PostgreSQL projects; two-session cases check both
orders of account changes versus ordinary-income insertion. Prior financial,
identity and audit checks remain applicable. P03.11/P03.12 are not completed by
these targeted checks.
