# Schedule recurrence generation in Dokploy

Run the existing one-shot job daily inside the running API container. It generates
pending occurrences for all eligible verified owners; it never confirms payments
or posts money. This guide is not evidence of an installed schedule.

## Prerequisites

- Use authorized staging with synthetic data first. A manual run can persist
  pending occurrences for every eligible owner, not just the test account.
- Publish and deploy an image built with the packaged
  `/usr/local/bin/cuenora-recurrence-job`. The alpha.5 Dockerfile omitted this
  binary; do not configure a schedule against alpha.5 or assume a newer tag has
  it without checking. Publishing/deployment need separate maintainer action.
- Apply the existing schema through migration 0018 at minimum; the current API
  expects the full current schema. This job does not run migrations.
- The API must already use its restricted database login, not the administrator.
  Application jobs execute in the API container and reuse its environment,
  including `CUENORA_DATABASE_URL`. Do not put credentials in the scheduled command.
  This reuses the API role, not a separate, more restricted job identity.

## Check the binary without database writes

In Dokploy, open the **API container terminal**, not the PostgreSQL terminal or
host terminal, and run:

```sh
/usr/local/bin/cuenora-recurrence-job --help
```

If the binary is missing, stop and deploy an image containing it. Do not compile
inside the running container, install cron there, change the API entrypoint or
replace the API command with the job.

## Run once before scheduling

Only after accepting the pending-occurrence side effects on staging:

```sh
/usr/local/bin/cuenora-recurrence-job
printf 'exit=%s\n' "$?"
```

Expect `exit=0` and either:

- `status=completed rules_processed=N occurrences_created=N rules_failed=0`, or
- `status=already_running` when another job owns the database advisory lock.

A zero-rule run proves startup/access but not generation or write permissions.
Use an existing synthetic eligible rule and check its pending occurrences through
normal API reads to demonstrate generation; check that balances are unchanged.
Do not print record contents, credentials, tokens or private identifiers in reports.
An overlapping no-op does not prove that the active job completed successfully.

Exit 1 means a job/storage/per-rule failure; exit 2 means arguments/configuration.
`status=completed` with nonzero `rules_failed` is a failure, not success. Inspect
before retrying. Earlier successful rules may already have committed.

## Configure the application schedule

In the API application's Dokploy **Schedule Jobs** settings, create an application
job targeting that running API container:

| Field | Value |
| --- | --- |
| Name | `cuenora-recurrence-daily` |
| Command | `/usr/local/bin/cuenora-recurrence-job` |
| Cron expression | `15 2 * * *` (daily at 02:15 in the scheduler's timezone) |

Verify the scheduler's effective timezone before enabling: the product example
uses UTC, but do not assume your Dokploy installation uses it. Do not change the
server timezone blindly. If UTC scheduling is unavailable, document the actual
zone and daily execution time; rule dates still use each rule's own IANA zone.
A daily run may generate a due slot later than its local midnight.

Dokploy executes Application/Compose scheduled commands inside a running container.
Use an Application job for an Application deployment, or the corresponding API
service Compose job for a Compose deployment. Do not use a host/server job or
mount the Docker socket into the API container.

After saving, use manual execution and inspect its execution log and exit status.
Then observe at least one automatic daily execution. A saved cron expression or
manual success alone does not prove scheduling works. Record aggregate counts,
exit status and execution timestamp/timezone, not private rule/account data.

## Failure and rollback

The job uses a PostgreSQL session advisory lock on the same connection used for
generation. Overlap safely skips; do not add a different lock key or bypass this
job. Existing tests cover overlap and cancellation, but scheduling adds no new
staging failure-injection evidence.

This minimal route depends on the API container being running. Inspect failures
and missed runs after deployments; active rules catch up on later successful
passes, while paused periods are not generated. API health is not job monitoring.

To stop automatic generation, disable/remove only this Dokploy schedule. Keep the
API running and preserve existing rules/occurrences. Disabling the schedule does
not undo pending occurrences already generated. Never delete records as rollback.
A separately deployed job would require its own secrets, network and healthcheck
configuration; it is intentionally not introduced here.

## Completion evidence still required

- [ ] Published image revision/digest includes the binary and help check passes.
- [ ] Manual staging generation succeeds with the restricted role and an eligible rule.
- [ ] Pending occurrences verified and account balances unchanged.
- [ ] Scheduler timezone recorded; automatic execution observed with exit 0.
- [ ] Failure/missed-execution monitoring defined before production use.

The local packaging change and CLI image tests do not check off these deployment
items. Do not mark P07.09 deployment or Phase 14 production readiness complete
from this runbook alone.

See [job semantics](../product/recurrence-job.md) and
[Dokploy Schedule Jobs](https://docs.dokploy.com/docs/core/schedule-jobs).
