# ADR-025: Verified-owner account metadata and bounded cursor reads

- **Status:** Accepted; own/independent checks passed; native review pending
- **Date:** 2026-10-29
- **Roadmap:** P05.04, with operation-specific P04.06/P04.10 proof

P05.05 subsequently extends only the metadata-only GET shape with an exact current
balance; see [ADR-026](026-exact-account-balances.md). The original P05.04 decision
below remains historical context.

## Decision

The maintainer approved listing active and archived accounts together, with the
existing five metadata fields and no balances. Require bearer identity and current
verified email, preserving the broad financial-access policy. Profile-zone
confirmation is unnecessary for this undated read. No client owner hint, archive
filter or archive/restoration workflow is introduced.

`GET /v1/accounts` uses a cursor, default50/max100 rows and stable ascending
`created_at,id` order. The cursor identifies the last returned owned account; it
grants no authority. Look up the cursor within the authenticated owner and use
that same predicate in the page query. Missing/foreign cursors share a generic
invalid-request response. Fetch at most `limit+1` rows to decide whether another
page exists; do not fetch all records and truncate in application memory.

Use one short read-only repeatable-read transaction for verified-email lookup,
owned cursor lookup and page selection. Local statement/lock timeouts bound waits.
The page sees one database snapshot without write locks, financial side effects
or timestamp updates. Separate requests may observe newly committed records;
the API does not promise a long-lived snapshot across an entire traversal.

Reuse existing bearer/request-ID/no-store/typed-error infrastructure, the owner
index and metadata storage. Enable the already locked Axum query feature for
strict Serde query parsing rather than inventing a URL parser or adding a separate
query/UUID dependency. Retain every existing locked dependency version. Existing
POST/create/opening behavior is unchanged.

## Tradeoffs and boundaries

Including archive state avoids silently hiding owned records, but does not settle
archive mutation rules. Returning raw stored names preserves legacy/imported
values without write-on-read normalization. The creation date/UUID ordering is
stable even when names duplicate or change; it is not UI drag-and-drop ordering.

A read transaction adds a few bounded round trips but keeps verification, cursor
and results coherent. It is not a global lock or cross-request snapshot. The
existing owner index suffices for this first slice; later measured query plans
may justify a separate reviewed index migration, not an implicit schema change.

An unavailable read returns a safe503, never a misleading empty collection. See
the [listing contract](../../product/account-listing.md) for query/body bounds,
errors and pagination. Balances, archive workflows, other resources and email
verification delivery remain outside this feature.
