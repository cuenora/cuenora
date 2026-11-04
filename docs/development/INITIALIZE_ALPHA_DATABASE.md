# Initialize an empty alpha PostgreSQL database

Initialize a **disposable, empty staging database** for
`carfdev/cuenora:0.1.0-alpha.2`. The API image does not apply migrations at startup.
This is a one-time bootstrap, not a migration runner for existing databases.

## Prerequisites

- Use the **server terminal**, not the PostgreSQL container terminal.
- The server must have Bash, Docker, curl and tar.
- Confirm that the target database is `cuenora` and has no tables in `public`.
- The commands below use the `postgres` role for schema initialization only.
  This is not a recommendation to use a superuser as the API runtime identity.
- Use synthetic data only. Do not publish PostgreSQL's port to the Internet.
- Stop concurrent bootstrap/deployment attempts while initializing the database.

The SQL is pinned to commit `51adc51d4b76ec008d6c6b694273dc6926d842f6`, the source
revision published as alpha.2. It contains migrations `0001`–`0023`.

## Run once from the server

Find the current PostgreSQL container name:

```sh
docker ps --format '{{.Names}}\t{{.Image}}' | grep -i postgres
```

Replace `REPLACE_WITH_POSTGRES_CONTAINER_NAME` below with that name. Review the
database name before running. Copy the **entire block**, including its final
`BASH` line. The placeholder check prevents accidental execution without editing.

```bash
bash <<'BASH'
set -euo pipefail

container='REPLACE_WITH_POSTGRES_CONTAINER_NAME'
revision='51adc51d4b76ec008d6c6b694273dc6926d842f6'

test "$container" != 'REPLACE_WITH_POSTGRES_CONTAINER_NAME' || {
  echo 'Set the PostgreSQL container name before running.' >&2
  exit 1
}

workdir=$(mktemp -d /tmp/cuenora-init.XXXXXX)

curl --fail --silent --show-error --location \
  "https://codeload.github.com/cuenora/cuenora/tar.gz/$revision" \
  --output "$workdir/source.tar.gz"

tar -xzf "$workdir/source.tar.gz" -C "$workdir"

files=("$workdir/cuenora-$revision/backend/migrations/"*.sql)
test "${#files[@]}" -eq 23

{
  printf '%s\n' "DO \$\$ BEGIN IF EXISTS (
    SELECT 1 FROM pg_catalog.pg_tables WHERE schemaname = 'public'
  ) THEN RAISE EXCEPTION 'Database is not empty'; END IF; END; \$\$;"

  for file in "${files[@]}"; do
    cat "$file"
    printf '\n'
  done
} | docker exec -i "$container" \
  psql -X -U postgres -d cuenora \
  --set=ON_ERROR_STOP=1 --single-transaction --file=-

echo 'Database initialization completed.'
echo "Downloaded sources: $workdir"
BASH
```

The check and all migrations run in one transaction. An SQL error stops execution
and rolls back that transaction. Existing tables cause rejection, not deletion.
The commands do not print passwords. Downloaded sources remain in the printed
temporary directory for inspection.

## Verify before continuing

Successful output ends with `Database initialization completed.` The wrapper
prints this only after psql exits successfully; psql need not print its implicit
transaction-control statements. Do not rerun a successful bootstrap.

From the PostgreSQL container terminal in Dokploy, run this single-line,
read-only check:

```sh
psql -X -U postgres -d cuenora -Atc "SELECT count(*) FROM pg_catalog.pg_tables WHERE schemaname = 'public'; SELECT to_regclass('public.users'), to_regclass('public.session_access_tokens'), to_regclass('public.password_recovery_challenges');"
```

The count must be nonzero and all three table references must be present.
Then verify that the API has the correct private database connection URL and
that authentication can query the schema. `/v1/health/live` alone does **not**
check database readiness.

If initialization fails, retain the error, inspect the target database and
resolve the cause before retrying. Do not delete tables or volumes as a recovery
shortcut. This bootstrap creates no migration-history ledger; future schema
updates need a separately controlled migration procedure.

See [psql transaction/error behavior](https://www.postgresql.org/docs/18/app-psql.html)
and [runtime configuration](SETUP.md#validated-runtime-environment).
