# Account movement storage

P03.08 adds `backend/migrations/0006_account_movements.sql`: effective, append-only
account postings. It does not post transactions automatically or expose an API.

| Field | Storage contract |
| --- | --- |
| `id` | Generated UUID primary key. |
| `user_id` | Existing owner; restrictive deletion. |
| `transaction_id`, `account_id` | Required event/account links, each with a composite owner foreign key. |
| `signed_amount_minor`, `currency` | Exact integer hundredths, symmetric signed `Money` bounds; SEK/EUR/USD. Zero is representable for zero opening balances. |
| `occurred_on` | Posting economic date, years 0001–9999. |
| `created_at` | UTC audit instant with microsecond precision, years 0001–9999; statement-time default. |

Indexes support owner/account/date history, owner/date queries and event/owner
lookup. Currency, date and amount are posting snapshots, not generated from the
current event. There is no movement lifecycle state: planned items do not belong
here. Balances will sum all effective signed postings per account/currency,
including compensations; do not drop historical postings by joining the current
transaction state and filtering out `voided` events.

## Corrections preserve history

The approved [append-only decision](decisions/006-append-only-account-movements.md)
rejects UPDATE, DELETE and TRUNCATE with SQLSTATE `55000`, including no-op updates
and cascading truncation. An inverse posting can compensate an original while
both remain linked to the event. No unique event/account constraint prevents
later correction entries. Restrictive foreign keys prevent deleting referenced
accounts/events or reassigning their owner. Privileged DDL can disable guards;
this is not database-role hardening or authenticated access.

P05.13 must still define and implement atomic correction sets, reversal linkage,
replacement amounts, correction dates and audit history. Appending an arbitrary
negative row is not a validated reversal command. Changing the event's state
alone does not compensate its movements.

## Deferred financial guards

P03.10 adds [account currency compatibility and savings-income guards](financial-constraints.md).
Event/account membership, sign/amount consistency and posting counts belong to
future application posting/correction protocols (including P05.13), not P03.10.
The original movement migration checks row shape and ownership, not whether
postings match an event's financial meaning. It does not implement transfer
atomicity/idempotency, balance queries, future-date validation or protection against duplicate posting requests.

## Verify and recover

Apply in filename order using the [transactional procedure](identity-storage.md#apply-verify-and-recover).
Run `python3 tests/infrastructure/test_postgres_compose.py` against disposable
Compose projects. Tests cover exact signed storage, compensations, owned links,
append-only guards, index definitions and rollback after late DDL failure.
Reapplication fails without changing data. Disposable databases may be recreated;
existing postings require reviewed forward corrections/backup recovery, not drops.
