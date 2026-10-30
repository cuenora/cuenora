# Transaction event storage

P03.07 adds `backend/migrations/0005_transactions.sql`. It stores financial
business events, not account balances. That migration adds no API, movement or audit table.
P03.08 adds [account movement storage](account-movement-storage.md) separately.

| Field | Storage contract |
| --- | --- |
| `id`, `user_id` | Generated UUID; required existing owner, restrictive deletion and `(id, user_id)` reference target. |
| `kind` | `income`, `expense`, `transfer`, `opening_balance` or `interest_income`. |
| `state` | `posted` (default) or `voided`; no drafts or planned occurrences. |
| `occurred_on` | Civil `DATE`, years 0001–9999; not an audit instant. |
| `amount_minor`, `currency` | Exact integer hundredths, symmetric signed `Money` bounds and SEK/EUR/USD. Non-opening amounts are positive; openings can be signed. |
| `account_id` | Optional assigned account for income/expense; required receiving account for openings/credited interest, source for transfers. |
| Transfer fields | Required distinct destination and positive `received_amount_minor` with `received_currency`; absent on other kinds. Same-currency amounts must match. |
| `category_id`, `category_kind` | Optional global category; generated kind matches expense or income (including interest). Openings/transfers have no category. |
| `description` | Optional text; future transport validation must define input limits. |
| Audit fields | Microsecond `created_at`/`updated_at`, years 0001–9999; statement-time defaults, updates maintain `updated_at` explicitly. |

Accountless income/expense remains one business event without an implicit cash
account. A transfer stores the actual sent/received amounts, never a converted
aggregate; `20000` SEK minor units and `1795` EUR minor units remain distinct.
Composite foreign keys prevent either account reference from crossing owners.
The generated category discriminator cannot be supplied or overwritten by callers;
it reuses the [category kind target](category-storage.md). See PostgreSQL's
[generated-column contract](https://www.postgresql.org/docs/18/ddl-generated-columns.html).

## Storage is not a posting protocol

`posted` and `voided` are stored values, not implemented transition commands.
Future reporting must distinguish them. Editing/voiding with atomic movement
corrections and auditable history remains P05.13. This migration creates no
balance effect, performs no ownership authentication and grants no user access.
Category selection and bounded descriptions remain application responsibilities.

P03.10 adds account currency compatibility, savings ordinary-income restrictions
and the user/date index; see [financial constraints](financial-constraints.md).
Basic kind/amount/currency guards from this migration remain unchanged. Future-date validation must
use the confirmed profile zone when creating or explicitly editing a date, not
the database's `current_date`; see [date semantics](../product/dates-and-time-zones.md).
P05.08 adds this boundary for accountless income; other transaction operations remain pending.

The optional quoted FX rate is deferred until P06.01 specifies precision,
validation and discrepancy presentation. Transfer idempotency remains P06.07;
there is no placeholder key that falsely promises request fingerprint/retry
semantics. Movement storage is available in P03.08; minimal [audit storage](transaction-audit-storage.md) is available in P03.09.
Automatic posting/auditing and complete posting-protocol guards remain pending.

## Verify and recover

Use the [existing transactional migration procedure](identity-storage.md#apply-verify-and-recover)
and run `python3 tests/infrastructure/test_postgres_compose.py` from the root.
Tests cover exact storage, both states, accountless and transfer forms, ownership,
category compatibility, rejected-write snapshots and reapplication. A late owner
index collision rolls back the new table while preserving earlier data/schema.
For disposable tests recreate the database; populated deployments need reviewed
forward corrections and backup/restore plans, not a casual table drop.

## Accountless income creation (P05.08)

Implemented locally; native review skipped for this candidate by maintainer choice; publication pending: [HTTP contract](../product/accountless-income.md),
[ADR-029](decisions/029-accountless-income-retries.md). Migration
`0012_income_creation_keys.sql` adds permanent owner-scoped SHA-256 key/request
digests and an owned event reference, with no financial snapshots. Existing
transaction data is unchanged. Event, create audit and retry record are atomic;
no account movement is created. Editing/voiding and other posting workflows remain pending.

## P05.13 correction protocol

[Atomic corrections](../product/transaction-corrections.md) add partial edits and
voids for all five recorded kinds; own/independent verification passed, native review skipped for this candidate by maintainer choice.
Migration `0014_transaction_mutations.sql` adds immutable owner-scoped retry
keys and nullable linked inverse postings, without rewriting existing records.
Historical pending statements above describe the original migration boundary.
