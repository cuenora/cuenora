# Transaction audit storage

P03.09 adds `transaction_audit` in migration `0007_transaction_audit.sql`.
It records immutable metadata, not historical copies of financial records.
The maintainer approved field-name-only summaries to avoid sensitive duplication;
this deliberately does not support reconstructing previous transaction versions.

## Stored contract

| Field | Meaning |
| --- | --- |
| `id` | Generated UUID identifying the event, independent of its timestamp. |
| `user_id` | Existing owner; composite FK links the transaction to that owner. |
| `actor_id` | Existing user, constrained to equal the owner; no system actors yet. |
| `transaction_id` | Existing owned transaction, with restricted deletion. |
| `action` | Exactly `create`, `edit` or `void`. |
| `changed_fields` | Required one-dimensional text array of up to 11 recognized field names; empty arrays allowed. No null elements, values, snapshots or free text. |
| `created_at` | Server-default `TIMESTAMPTZ(6)` instant, years 0001–9999; non-finite instants rejected. |

Recognized names are `kind`, `state`, `occurred_on`, `amount_minor`, `currency`,
`account_id`, `destination_account_id`, `received_amount_minor`,
`received_currency`, `category_id` and `description`. `description` identifies a
field; its text is never copied into the audit record. Identity, owner, generated
columns and audit timestamps are not editable financial fields. Empty lists do
not claim a particular field changed. Duplicate names are not meaningful but
are not rejected; callers should provide distinct names.

UPDATE, DELETE and TRUNCATE are rejected at statement level, including empty
updates and cascading truncation. Rejected statements roll back the entire
transaction. The `(user_id, transaction_id, created_at)` index supports scoped
history queries. Multiple events can share a timestamp: it is not a sequence,
idempotency key or proof of ordering.

## Storage is not automatic auditing

There are no triggers recording transaction changes or movement corrections.
Existing transaction writes do not create audit rows. P05.13 must record accepted
changes and their audit metadata atomically, derive the actor from authenticated
identity, enforce action semantics, and reject client audit overrides. Storage
checks cannot establish that the listed fields actually changed.

As with other audit defaults, privileged SQL may supply an explicit timestamp.
The database stores an instant; session-dependent display is not canonical API
serialization. Database owners can disable triggers or change schema: append-only
guards are not protection against privileged administration. No roles, grants,
row-level security or data-erasure policy are added here.

## Verify and recover

Run `python3 tests/infrastructure/test_postgres_compose.py` against local Docker.
Tests use synthetic disposable databases, never personal or production records.
Apply migrations in order in a transaction; an index collision must leave no
partial audit table/function. Reapplication fails rather than silently masking
schema drift. No destructive down migration is supplied; do not remove populated
audit history to roll back application code.
