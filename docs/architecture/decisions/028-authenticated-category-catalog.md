# ADR-028: Authenticated global category catalog

- **Status:** Accepted
- **Date:** 2026-10-29
- **Roadmap:** P05.07

## Context

The immutable built-in catalog is shared reference data, not a user's financial
records. Storage already rejects mutations and incompatible transaction-category
kinds. The API needs to expose these existing categories without implementing
transaction creation or custom categories.

## Decision

Expose `GET /v1/categories` behind the existing bearer identity middleware.
A valid session is required; verified email and a confirmed profile zone are not.
The maintainer explicitly approved this boundary. Financial endpoints retain
their separate verification, ownership and date requirements.

Return all ten stored categories, ordered by UUID, with `id`, `code`, `kind`, and
`name`. Do not duplicate migration seeds in runtime code. Provide no filtering,
pagination or mutation endpoints. Reject nonempty queries and request bodies.
Use read-only SQL, bounded lock/statement timeouts and safe errors.

## Consequences and alternatives

Authentication keeps reference reads consistent with protected resource routes,
but requires a database-backed identity lookup even for public-like reference
data. A public endpoint would avoid that lookup, but was not selected. Allowing
pending users to read reference data grants no financial access.

Category-kind compatibility stays in the existing generated-kind/composite-FK
storage constraint. Future transaction use cases must surface suitable input
errors; this slice adds no unused validation interface or transaction endpoint.

See the [catalog contract](../category-storage.md#read-only-http-catalog-p0507).
