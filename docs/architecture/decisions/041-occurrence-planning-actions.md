# ADR-041: Serialized planning actions with permanent retry results

- **Status:** Accepted
- **Date:** 2026-11-01
- **Roadmap:** P07.11

## Context

An existing occurrence can be skipped or postponed independently of its rule.
The original calendar slot must remain stable and no financial posting is made.
Repeated mobile requests must not undo later changes or advance a date twice.
The maintainer approved strictly later due dates, including beyond rule end.

## Decision

Expose explicit verified-owner skip and postpone POST actions. Reuse recurrence's
owner-first transaction boundary, then lock the occurrence. Skip changes planned
to skipped; postpone keeps planned and changes due_on only. New terminal actions
conflict. No today-derived date or profile-zone gate applies to these planning actions.

Use one permanent owner-wide action-key namespace, independent of confirmation
and rule creation. Store key/request hashes, owner/occurrence linkage, action and
original result due date atomically with the mutation. Matching retries return the
original action result even after later actions; they do not return mutable state
or restore prior state. Preserve existing terminal database guards.

## Consequences and alternatives

Permanent replay requires an additive table and growing metadata, but survives
process restarts and later confirmation. State-only idempotence was rejected:
a successful postponement would reject its own retry after due_on advanced, and
an older retry could overwrite newer intent. A generic occurrence PATCH was not
chosen because it obscures terminal skip and risks expanding editable terms.
No stored financial values or financial audit changes are introduced.

## Verification

Public HTTP tests cover exact action effects, date bounds, replay, ownership,
terminal conflicts, generation preservation, races and rollback. Migration tests
cover populated history and immutable retry constraints. See the
[contract](../../product/recurrence-occurrence-actions.md) and
[execution record](../../plans/active/2026-11-01-occurrence-actions.md).
