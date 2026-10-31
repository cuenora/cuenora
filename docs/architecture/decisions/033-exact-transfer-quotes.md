# ADR-033: Exact entered transfer quotes

- **Status:** Accepted; implemented locally, own/independent checks passed
- **Date:** 2026-10-31
- **Roadmap:** P06.06

## Context

The accepted [transfer contract](../../product/currencies-and-transfers.md) allows
an optional entered directional rate. Actual amounts determine postings; a rate
discrepancy is informational. Existing no-quote retries are permanent, so extending
the request must not change their hashes or response shapes.

## Decision

Store only the canonical entered decimal in nullable `transactions.quoted_rate`
text, added by migration `0016_transfer_quotes.sql`. Do not infer historical rates
or store computed comparison snapshots. Domain parsing accepts positive ASCII
decimals with at most twelve fractional digits and canonicalizes zeros. Checked
`i128` coefficient arithmetic reuses `Money::from_minor_ratio` for one half-even
quantization in destination minor units; overflow rejects without effects.
Database constraints enforce canonical shape, transfer-only use and these bounds.

Creation/history include `quoted_rate` and `rate_comparison` only when a quote
exists. The latter provides indicative received amount, actual-minus-indicative
difference and a mismatch boolean in the received currency. Native clients can
present the approved warning; no mobile UI exists in this implementation.

Retain the exact no-quote fingerprint bytes. Append a domain-separated canonical
quote component only when supplied. Replays reconstruct the original quote
comparison from the matching original request, independently of mutable history.

Existing corrections/voids retain the quote. History recomputes comparisons using
current amounts; corrections that overflow reject atomically. Rate editing is
not added. Existing minimal audit and append-only postings remain unchanged.

## Tradeoffs

Canonical text avoids a new decimal dependency or database-specific numeric
serialization. Explicit checked integer bounds can reject syntactically valid
but unrepresentable quotes; this follows the accepted overflow policy. PostgreSQL
exact `numeric` arithmetic is used only by constraints, never floating point.

Omitting new fields without a quote preserves existing clients and acknowledged
responses. Recomputed comparison metadata avoids redundant financial snapshots,
but every correction must preserve representability. Generic decimal storage
without domain validation would permit values the application cannot safely read.

## Verification and delivery

See the [execution plan](../../plans/active/2026-10-31-quoted-transfers.md) for
observed checks. Local implementation is not publication or Phase 06 completion.
