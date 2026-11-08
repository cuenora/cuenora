# ADR-059: Owner-bound reads and native account browsing

- **Status:** Accepted for P11.04 implementation
- **Date:** 2026-11-08
- **Roadmap:** P11.04

## Context

AccountRepository.list already supplies metadata, exact posted balances and cursor
pagination. There is no single-account GET endpoint. Login returns opaque credentials,
not a trustworthy owner identity. The maintainer explicitly left account activity to
P11.06; these screens must not imply activity, forecasts or management are implemented.

## Decision

Add an authenticated shared read adapter over existing repositories, not another HTTP
contract. Within AuthSession.authorized, read the current server profile, require
verified email, and call the existing account listing with that owner/current bearer.
Bind each reader to its first verified owner; reject another identity before reading
more financial records. Renewal and logout retain the existing serialization domain;
a 401 removes local authority and no read is automatically retried.

Keep presentation state in Android, not a new shared presentation framework. The
native browser is created inside the signed-in subtree and permanently retired on
disposal. It never persists records, credentials or selection to saved instance state.
One in-flight read is allowed; duplicate read actions do not queue. Disposal rejects
late results. Cancellation releases the busy state so a subsequent read can proceed.
This does not implement or mark P10.10 complete.

The selected list record supplies detail metadata and balance. Describe it as the
last successful read, not live data. Returning preserves pages/list position; explicit
refresh returns to the list and replaces the old snapshot only on successful load.
During refresh the old balances are unavailable. Failed additional pages retain prior
rows and their cursor for explicit retry; denied access clears all displayed records.
Reject cursor cycles and owner-mixed pages and deduplicate overlapping row identities.

Amounts use integer hundredths rendered directly with two decimal places and an
explicit currency code, including signs and the full symmetric Long range. No binary
floating point, implicit currency conversion or aggregated grand total is introduced.
Use the existing Compose theme, controls, amount and notice components.

## Consequences and alternatives

List-record detail needs no new API or backend work, at the cost of a read-time snapshot
and whole-list refresh instead of a dedicated single-account refresh. A dedicated GET
can be considered when future detail workflows genuinely need it; it is not invented
for this task. Refresh intentionally discards loaded pages, selection and stale balance
values. Activity, management, pending payments and additional creation stay out of scope.

Shared MockEngine tests pin identity, bearer, denied/session failures and pagination
forwarding. Native state/Compose tests pin exact display, paging/retry and Back. Independent
security/order probes cover the new session integration. Isolated tests do not establish
hosted E2E, API23 runtime or complete assistive-technology compatibility.
