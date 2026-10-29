# Create an account

**Roadmap:** P05.02 · **Status:** Implemented locally; own and independent checks passed; review/delivery pending

`POST /v1/accounts` creates account metadata, not a balance or opening movement.
It requires an active bearer session, verified email and `Idempotency-Key`.
The verification email/confirmation runtime is still pending; there is no
production or development bypass to this gate.

## Contract

Send strict JSON (8192-byte maximum; unknown and duplicate fields rejected):

```json
{"name":"Daily","account_type":"normal","currency":"SEK"}
```

Names follow the [domain policy](accounts.md): trim outer Unicode whitespace,
keep 1–255 Unicode scalar values, reject ASCII controls, preserve other text and
allow duplicate names. Types are `normal`/`savings`; currencies `SEK`/`EUR`/`USD`.
Never send `user_id`, status, balance, opening amount or financial dates.
A confirmed profile zone is not required for this undated metadata operation.

Success and replay return 201, `Cache-Control: no-store`, a fresh `X-Request-ID`
and the same original five-field account JSON (including after later renaming):

```json
{"id":"00000000-0000-0000-0000-000000000001","name":"Daily","account_type":"normal","currency":"SEK","status":"active"}
```

Use a case-sensitive key of 1–128 ASCII letters/digits/`-`/`_`.
Generate a random unique key for each intended creation; keep the same key while
retrying an uncertain response. Duplicate/missing/invalid key headers return 400.
Keys are scoped to the authenticated owner and account creation, not to a session.
The same key and normalized name/type/currency replay the original result;
changed content returns 409 `idempotency_conflict`. Different keys may create
accounts with identical names. Records have no TTL and last for the account lifetime.

| Failure | Status / code |
| --- | --- |
| Invalid/missing/expired/revoked bearer | 401 `invalid_access_token` |
| Unverified owner | 403 `email_not_verified` |
| Invalid body or key | 400 `invalid_request` |
| Oversized body / unsupported media type | 413 / 415 `invalid_request` |
| Same owner's key with different normalized content | 409 `idempotency_conflict` |
| Unavailable identity lookup | 503 `authentication_unavailable` |
| Transaction/lock/commit failure | 503 `account_creation_unavailable` |

Errors use the existing safe four-field envelope and no-store policy. Entropy
failure follows the existing 503 `request_id_unavailable` exception. No database
or submitted value is echoed. A 503 or disconnected response can follow an
uncertain commit; retry the same key. Rejected validation/authorization/conflicts
write nothing. Account and idempotency insertion failures roll back together.

## Storage and verification

Migration `0010_account_creation_keys.sql` adds a creation-only owner/key-digest
primary key and an owned account foreign key, plus the original name/type/currency.
There is no cleanup job or TTL. Do not drop populated records to retry deployment;
use a forward repair or controlled restore. There is no API for deletion here.

Run the [isolated PostgreSQL harness](../development/SETUP.md) for HTTP tests of
ownership, rejected writes, same/different keys, concurrency, rollback and lifetime
replay. Seeded verification tests the gate only, not delivery or confirmation.
Opening movements and their date/zone/retry contract remain P05.03.
