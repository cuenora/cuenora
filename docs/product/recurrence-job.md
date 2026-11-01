# Daily recurrence job (P07.09)

`cuenora-recurrence-job` runs **once**, generating eligible due planning occurrences
across verified owners. Run it daily with an external scheduler; it neither starts
the HTTP API nor installs a scheduler. It never confirms payments or posts money.

## Run and schedule

Apply migrations through 0018 and export `CUENORA_DATABASE_URL` through your
scheduler's private environment/secret mechanism. The executable does not load
`.env`; never put credentials in command arguments, cron lines or logs.

From `backend/`:

```bash
cargo run -p cuenora-api --bin cuenora-recurrence-job --locked -- --help
cargo run -p cuenora-api --bin cuenora-recurrence-job --locked
cargo build -p cuenora-api --bin cuenora-recurrence-job --release --locked
```

For a deployed binary, an example cron entry is:

```cron
15 2 * * * /opt/cuenora/cuenora-recurrence-job
```

Configure the scheduler itself to use UTC for this example and provision its
private database environment separately. This is operational guidance, not an
installed cron entry or deployment. Use a role permitted to read rules/users and
perform generation's existing planning writes; no administrator rights are needed
by the application. No production database is accessed by local tests.

A daily invocation can materialize a slot later than its local midnight (up to
about one scheduler period). Each rule still uses its own local **today**, not the
scheduler's UTC date. Active downtime catches up; user-paused intervals do not.
The existing `cargo run -p cuenora-api --locked` still selects the HTTP API.

## Results and failures

| Exit | Meaning |
| --- | --- |
| 0 | Complete pass with no rule failures, overlapping run skipped, or help |
| 1 | Database/job failure or one or more failed rules; retry is safe |
| 2 | Invalid arguments or missing/invalid database configuration |

Only `--help` and `-h` are accepted. Unknown/extra arguments reject before database
access. Output contains aggregate counts, never SQL diagnostics, credentials,
owner/rule IDs or financial terms. A completed pass prints
`status=completed rules_processed=N occurrences_created=N rules_failed=N`;
an overlapping pass prints `status=already_running` and changes nothing.
A rule can succeed with zero new slots. A partial failure preserves committed
successful rules, rolls back the failed rule and returns exit 1. A fatal discovery
or connection failure may return only a generic unavailable error. No in-pass
retry loop is introduced: retry manually or on the next scheduled invocation.

## Concurrency and limits

The database-scoped PostgreSQL session advisory key `(0x4355454e, 709)` is stable
across replicas. All invocations must use this job, not a substitute lock key.
One dedicated SQLx connection holds exclusion **and executes all discovery and
per-rule transactions**. It is marked close-on-drop before lock acquisition;
normal completion unlocks and closes it. Failure/cancellation discards it, so a
session lock cannot leak into the pool. Connection loss also removes generation's
write capability. PostgreSQL releases its locks when it detects session closure;
network failure detection can delay a new run, but cannot allow that lost session
to continue writing.

Discovery uses 100-row UUID keyset pages and an initial upper UUID boundary. Rules
created behind the cursor or above that boundary are handled on a later pass;
there is no global snapshot of all rules. Paused/unverified rows are not selected.
Every selected rule revalidates verified ownership and current state under the
existing owner → rule locks, even if state changes after discovery.

One transaction per rule retains the existing 2-second lock and 5-second statement
timeouts. Memory is bounded by a page plus P07.08's date-bounded slot vector; a pass
can still take substantial time across many rules. No parallel per-owner writes,
new migrations/dependencies, configurable batch flags or posting are introduced.

See [generation](recurrence-generation.md), [ADR-039](../architecture/decisions/039-one-shot-recurrence-job.md)
and the [execution record](../plans/active/2026-11-01-recurrence-job.md).

## Verification

Ten disposable-PostgreSQL groups cover discovery, overlap, page boundaries,
per-rule rollback, cancellation, owner/state revalidation and terminated sessions.
A loopback-only protocol gate exercises cancellation after advisory acquisition
without delivering its result. Two process-level CLI groups cover help/arguments/
configuration and generic unavailable-storage errors. Existing generation/lifecycle
regressions and 80 baseline/candidate HTTP responses remain unchanged.
