# Due-occurrence generation (P07.08)

`cuenora_api::recurring_rules::generate_due` creates missing eligible expense
occurrences as `planned`, through **today inclusive in the rule's own time zone**.
It recovers active backlog but never posts expenses, movements or financial audit.
No generation HTTP route or scheduler is installed. The [P07.09 one-shot job](recurrence-job.md)
invokes generation under external daily scheduling.

## Application boundary

The caller supplies the database pool, a server-derived `AuthenticatedIdentity`,
rule ID and trusted clock closure, normally `|| std::time::SystemTime::now().into()`.
Never obtain the clock from a client-supplied date. The clock is evaluated once,
after acquiring owner and rule locks, so waiting across midnight cannot use a
pre-lock date. The return value is the count of newly inserted occurrences;
retries normally return zero, not the previous count or pending-payment total.

Unverified owners reject; missing/foreign rules return the existing opaque
not-found error. This is a backend application interface, not permission to expose
arbitrary identities, clocks or generation horizons over HTTP.

## Eligibility and preservation

- Paused rules do nothing, including no progress or audit-time change.
- Active rules search strictly after inclusive `generated_through`, using the
  original `starts_on` anchor and existing pure calendar calculator.
- The horizon is the earlier of local today and inclusive `ends_on`. Coverage is
  advanced only through that horizon: extending an end later can recover backlog.
- Completed half-open pause intervals exclude slots; the resume day is eligible.
  Calendar-edit coverage remains authoritative. Active downtime is not a pause.
- New rows snapshot current exact expense terms, including optional account and
  category. Retained archived-account references are planning only, not posting
  permission. Every new row starts with `due_on = scheduled_on` and no transaction.
- Existing slots are never updated, including overdue, postponed, skipped or
  confirmed occurrences. Their identities, terms and dates remain unchanged.

## Atomicity and bounds

Generation reuses P07.07's verified owner → rule locks, READ COMMITTED isolation,
2-second lock timeout and 5-second statement timeout. A set-based insert filters
native PostgreSQL pause exclusions and uses the unique `(rule_id, scheduled_on)`
constraint with `ON CONFLICT DO NOTHING`. Insertion and monotonic progress commit
in one transaction; a failed statement cannot leave partial rows or coverage.
A covered/repeated horizon does not change `updated_at`.

Calendar enumeration is bounded by years 0001–9999 (at most 521,723 weekly slots)
and uses one insertion statement, not one database round trip per date. This
operation is per rule, not an all-owner batch. The [P07.09 job](recurrence-job.md) provides overlap admission and failure status;
external scheduling remains an operational responsibility.

## Verification

Ten application-interface PostgreSQL groups cover retries/concurrent calls,
rule-local today, future dates, all cadences, pause/end/edit boundaries, retained
snapshots, archived references, owner/verification guards and late-write rollback.
The disposable infrastructure harness runs ignored database tests explicitly:

```bash
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_recurring_generation_against_disposable_postgres -v
```

See the [lifecycle scenarios](recurrence-lifecycle.md#acceptance-scenarios-for-later-implementation),
[storage constraints](../architecture/recurrence-storage.md), and
[execution record](../plans/active/2026-11-01-due-occurrences.md).
