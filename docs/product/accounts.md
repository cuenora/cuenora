# Account domain model

P05.01 models account metadata in `cuenora-domain`, independently of HTTP,
SQLx, a database, a clock or UUID generation. It does not create stored accounts.

## Fields and boundaries

| Domain value | Meaning |
| --- | --- |
| `AccountId` | Opaque 16-byte UUID identity supplied by an outer layer. |
| `AccountOwnerId` | Distinct opaque owner UUID identity; not an authorization credential. |
| `AccountName` | Validated display name; duplicates are allowed. |
| `AccountType` | `Normal` or `Savings`, corresponding to storage `normal` or `savings`. |
| `Currency` | Existing `Sek`, `Eur` or `Usd` domain value. |
| `AccountStatus` | `Active` or `Archived`; storage uses NULL/non-NULL `archived_at`. |

Identifiers preserve bytes without generating or validating a UUID version.
Status represents business state, not an archive timestamp or an archive workflow.
Private account fields expose read-only accessors; construction accepts a validated
name. There is deliberately no balance, opening amount or interest configuration.

## Name policy

1. Trim leading and trailing Unicode whitespace using Rust `str::trim`.
2. Reject an empty result, then any remaining ASCII control character (U+0000–001F
   and U+007F), then names longer than 255 Unicode scalar values, not UTF-8 bytes
   or displayed grapheme clusters.
3. Preserve internal spaces, case and remaining Unicode exactly. Do not normalize
   Unicode or impose uniqueness.

For example, `"  Savings  "` becomes `"Savings"`; `"My  Savings"` keeps both
internal spaces. Tabs/newlines at the edges are trimmed, but internal ones are
rejected. Canonically equivalent Unicode spellings remain distinct strings.
Errors contain no supplied name.

The [storage guard](../architecture/account-storage.md) remains unchanged and is
not a canonicalization mechanism. Legacy/imported rows may pass that guard without
being trimmed; a future persistence adapter must explicitly handle that boundary,
not silently rewrite stored names on reads.

## Scope and verification

Account creation, owner authorization and verified-email gates, opening movements,
balance queries, rename/archive workflows and savings posting restrictions remain
later roadmap tasks. Representing an owner here does not implement P04.06/P04.10.
No API or database mapping is implemented by this slice.

From `backend/`, run `cargo test -p cuenora-domain --test account --locked`.
The public-interface tests cover metadata, both types/statuses, all currencies,
UUID-byte preservation, duplicate names, whitespace, Unicode, controls and bounds.
