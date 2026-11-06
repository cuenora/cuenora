# Run the API with a restricted PostgreSQL role

Use a dedicated runtime login instead of the PostgreSQL administrator. This
runbook covers the current 23 migrations and table-level operation grants, not
column-level least privilege or a production security certification.

## Scope and prerequisites

- Use only an explicitly authorized staging database with synthetic data.
- Keep PostgreSQL private, with no host-published database port. A private network
  does not replace authentication or application ownership checks.
- These examples assume database `cuenora`, schema `public`, and a new role
  `cuenora_api`. Review them before adapting to another installation.
- Preserve the current API connection secret securely for rollback. Never paste
  passwords, full connection URLs, tokens or personal records into reports.
- Keep migrations and object ownership under the administrator, not the runtime
  login. This procedure does not change ownership or run migrations.
- Inspect database/schema ACLs and role memberships before reusing an existing
  role. The role creation below intentionally fails if the name already exists.
- Revoke PUBLIC privileges only after confirming this database is dedicated to
  Cuenora; the revocation affects other non-administrator clients too.

## 1. Inspect before changing

Open `psql -U postgres -d cuenora` in the PostgreSQL container terminal. SQL
belongs at the `cuenora=#` prompt, not at the shell prompt.

```sql
SHOW server_version;
SELECT datname, pg_get_userbyid(datdba) AS owner, datacl
FROM pg_database WHERE datname = current_database();
SELECT nspname, pg_get_userbyid(nspowner) AS owner, nspacl
FROM pg_namespace WHERE nspname = 'public';
SELECT rule_number, type, database, user_name, address, auth_method, error
FROM pg_hba_file_rules ORDER BY rule_number;
```

An empty ACL field means default privileges, not no privileges. `NOINHERIT`
does not remove permissions granted to `PUBLIC`. On the observed PostgreSQL 18
installation, PUBLIC had schema USAGE but not CREATE; database defaults included
CONNECT and TEMPORARY. Do not assume these defaults on upgraded installations.

## 2. Create the runtime login

Execute this psql command **on its own**, then press Enter:

```text
\set ON_ERROR_STOP on
```

Then run the SQL:

```sql
CREATE ROLE cuenora_api LOGIN
    NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE cuenora_api IN DATABASE cuenora
    SET search_path = pg_catalog, public;
```

Set a new password interactively, without embedding it in SQL or shell history:

```text
\password cuenora_api
```

Store it in a password manager. If the login already exists, stop and inspect it
rather than ignoring the error or resetting credentials blindly.

## 3. Grant only the current runtime operations

The block below is for the current schema. Do not replace it with blanket ALL or
DELETE grants. If a statement fails, issue `ROLLBACK;`, investigate, and do not
change the API secret. No record contents or object owners are changed.

```sql
BEGIN;
GRANT CONNECT ON DATABASE cuenora TO cuenora_api;
GRANT USAGE ON SCHEMA public TO cuenora_api;
GRANT SELECT ON TABLE public.categories TO cuenora_api;
GRANT SELECT, INSERT ON TABLE
    public.users, public.password_credentials,
    public.user_sessions, public.session_refresh_tokens,
    public.accounts, public.transactions,
    public.account_movements, public.transaction_audit,
    public.session_access_tokens, public.account_creation_keys,
    public.income_creation_keys, public.expense_creation_keys,
    public.transaction_mutation_keys, public.transfer_creation_keys,
    public.recurring_rules, public.recurring_occurrences,
    public.recurring_rule_creation_keys, public.recurring_confirmation_keys,
    public.recurring_occurrence_action_keys,
    public.savings_terms, public.savings_rate_periods,
    public.email_verification_challenges,
    public.password_recovery_challenges, public.password_reset_notifications
TO cuenora_api;
GRANT UPDATE ON TABLE
    public.users, public.password_credentials,
    public.user_sessions, public.session_refresh_tokens,
    public.accounts, public.transactions,
    public.recurring_rules, public.recurring_occurrences,
    public.email_verification_challenges,
    public.password_recovery_challenges, public.password_reset_notifications
TO cuenora_api;
GRANT DELETE ON TABLE
    public.email_verification_challenges, public.password_recovery_challenges
TO cuenora_api;
COMMIT;
```

All 25 tables need SELECT for the current readiness probe, including otherwise
write-oriented records. Categories remain read-only; movements, audit, permanent
retry keys and savings history have no UPDATE/DELETE grants. Challenge pruning
requires DELETE on the two challenge tables only.

No sequences are present in the current migrations. Existing functions normally
retain PUBLIC EXECUTE; inspect customized function ACLs before cutover. Recurrence
checks use `public.recurring_date_valid(date)` and
`public.recurring_exclusions_valid(datemultirange)`. Do not revoke function
execution globally without reviewing these dependencies.

Do not automatically grant access to every future table. Review runtime grants
with each future migration; never execute schema changes as `cuenora_api`.

For the dedicated database, remove inherited temporary-table creation:

```sql
REVOKE TEMPORARY ON DATABASE cuenora FROM PUBLIC;
```

If inspection found PUBLIC CREATE on schema public, remove it deliberately before
cutover. This was not present in the observed installation:

```sql
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
```

Do not grant database/schema CREATE, privileged role memberships, ownership,
TRUNCATE or runtime trigger-administration privileges.

## 4. Verify through the private network

At the psql prompt, replace the placeholder with the actual private Docker
hostname; keep the closing double quote:

```text
\connect "host=REPLACE_WITH_PRIVATE_DB_HOST port=5432 dbname=cuenora user=cuenora_api"
```

Use a fresh password-authenticated connection. A loopback connection may match a
local `trust` rule and therefore does not validate the password. Inspect rule
ordering; do not weaken authentication to make the test pass. Do not publish the
database port for testing.

```sql
SELECT current_user,
    has_database_privilege(current_user, current_database(), 'CREATE') AS db_create,
    has_database_privilege(current_user, current_database(), 'TEMP') AS db_temp,
    has_schema_privilege(current_user, 'public', 'CREATE') AS schema_create,
    has_table_privilege(current_user, 'public.categories', 'SELECT') AS category_read,
    has_table_privilege(current_user, 'public.categories', 'INSERT') AS category_insert,
    has_table_privilege(current_user, 'public.account_movements', 'INSERT') AS movement_insert,
    has_table_privilege(current_user, 'public.account_movements', 'UPDATE') AS movement_update,
    has_table_privilege(current_user, 'public.account_movements', 'DELETE') AS movement_delete;
```

Expect `cuenora_api`, then `f, f, f, t, f, t, f, f`. These sampled checks do not
constitute an exhaustive audit of memberships, function ACLs or write behavior.

### Terminal recovery

- Press `q` to leave the output pager (`(END)`).
- `cuenora-#` or `cuenora->` means incomplete SQL; use `\r` to clear the buffer.
- A multiline paste interpreted as one `\set` command has not executed the SQL
  inside that command. Run psql meta-commands separately from SQL; if necessary,
  paste SQL statements individually, leaving the transaction open until COMMIT.
- An unterminated `\connect` quote is an error, not a successful role switch.
  Check the reported connected user before proceeding.

## 5. Change only the API connection in Dokploy

1. Save the prior API secret securely for rollback.
2. In the API service, update `CUENORA_DATABASE_URL` with `cuenora_api` and the new
   password. Preserve the private hostname, port, database and existing URL
   parameters. Percent-encode special characters in URI credentials.
3. Do not change the PostgreSQL service bootstrap/admin credentials. Preserve
   administrator access for controlled migrations.
4. Redeploy only the API. Check both `/v1/health/live` and `/v1/health/ready` for
   HTTP 200 with exact body `OK`. Readiness checks reads, not write permissions.
5. Run the [transfer smoke](STAGING_TRANSFER_CONCURRENCY.md) only with an authorized
   verified synthetic staging user. It creates persistent records and confirms
   Europe/Stockholm; review those effects before consenting. Never blindly rerun
   after a failure or unknown outcome.

A separately deployed recurrence job reads its own `CUENORA_DATABASE_URL`;
changing the API service does not update that deployment. The merged
[Dokploy schedule guide](DOKPLOY_RECURRENCE_JOB.md) instead runs the job inside the
API container, reusing its environment and restricted role. Operator-reported
manual generation and the first automatic staging execution are now recorded in
that guide; no separate job credentials were provisioned for this route. A narrower
job role is not delivered by this runbook.

If cutover fails, restore the previous API secret and redeploy the API as a
bounded rollback; this temporarily restores the old privilege risk. Do not drop
roles/data, rerun bootstrap, or add blanket grants to silence errors. Investigate
before retrying. No rollback drill is claimed below.

## Operator-reported staging evidence — 2026-11-05

| Check | Observed result and boundary |
| --- | --- |
| Network | Operator reported a private Docker network and no published PostgreSQL host port; no independent network audit. |
| Role and password | Operator created the restricted role and committed the grants above. A private-hostname connection prompted for and accepted its password. |
| Effective permissions | Sampled checks matched the expected values above, including TEMP false after revocation. The dedicated-database scope was operator-confirmed. |
| API cutover | Operator reported updating the API connection and redeploying, followed by live/ready 200 with OK. No independent deployed-environment inspection. |
| Financial write smoke | Operator supplied successful helper output: one same-key event, four distinct events, exact 86.00/14.00 SEK balances and rejected-write snapshots. Synthetic accounts/opening/five transfers remain. |
| Password recovery | Operator supplied successful recovery-helper output: reset 204, replay/old-password/old-refresh rejection and new-password login. The operator separately confirmed receipt of the password-change notification. This changed the synthetic account password and revoked prior sessions; no independent mailbox/API reproduction. |

The source audit covered migrations and current runtime queries; this document
was not executed as an end-to-end script against isolated PostgreSQL. Staging
results were supplied by the operator, not independently reproduced by the agent.

At this date, the remaining checks were the broader effective-privilege review,
email verification, recurrence confirmation/actions, savings and network isolation.
The follow-up below supersedes that pending staging status. Generation and the
first scheduled run retain their separate scoped evidence in the
[schedule guide](DOKPLOY_RECURRENCE_JOB.md).

## Staging completion follow-up — 2026-11-06

**P14.03 is completed for the verified staging scope.** The operator supplied
read-only catalog results, and the agent compared them with the grants documented
above. The successful queries ran in read-only transactions ending in `ROLLBACK`;
this follow-up changed no grants, roles, passwords or business records.

| Check | Observed result |
| --- | --- |
| Runtime role attributes | LOGIN true; SUPERUSER, CREATEDB, CREATEROLE, REPLICATION, BYPASSRLS and INHERIT false |
| Role memberships | No direct or indirect membership in other roles |
| Database and schema | Database owned by `postgres`; CONNECT true, CREATE/TEMPORARY false. `public` owned by `pg_database_owner`; USAGE true, CREATE false; no other non-system schema listed |
| All 25 public tables | Owned by `postgres`; effective operations exactly matched the table-level matrix above, including read-only categories and DELETE only on the two challenge tables |
| Delegation and column grants | No table grant options or extra column-only privileges; no TRUNCATE, REFERENCES, TRIGGER or MAINTAIN privileges |
| All 15 public routines | Owned by `postgres`; SECURITY DEFINER false, runtime EXECUTE true and EXECUTE grant option false |
| Sequences and future grants | No public sequences and no custom `pg_default_acl` entries; native PostgreSQL defaults still apply |
| Actual deployed API connection | The running Backend's current `cuenora-data` IP matched a `pg_stat_activity` connection using `cuenora_api`. A separate local `postgres` session was not attributed to the API |
| Private networking | API and PostgreSQL detached from the general application network; intended-path controls succeeded and a temporary shared-network probe could not establish either tested connection. See the [network evidence](STAGING_API_PROXY.md) for scope and limitations |

The operator also explicitly confirmed rerunning the same pre-cutover scripts
**after** switching the Backend to `cuenora_api`, with the same successful results
for email verification, recurrence confirmation/actions and savings. These are
operator-confirmed repeat smokes: their new full outputs and exact script names
were not supplied for agent inspection. Do not present them as independently
reproduced runs or claim additional test counts. Earlier transfer, password
recovery and recurrence-generation evidence remains recorded separately.

This closes the agreed staging task, not an exhaustive PostgreSQL/platform or
column-level least-privilege certification. The agent did not connect to the
server or execute the grant procedure end-to-end against isolated PostgreSQL.
Production validation, backups/restore, upgrade migrations, monitoring and a
rollback drill remain separate. No additional runtime changes were needed to
obtain this evidence.

## References

- [PostgreSQL privileges](https://www.postgresql.org/docs/18/ddl-priv.html)
- [Authentication rule ordering](https://www.postgresql.org/docs/18/auth-pg-hba-conf.html)
- [psql commands and password handling](https://www.postgresql.org/docs/18/app-psql.html)
- [Initial empty-schema bootstrap](INITIALIZE_ALPHA_DATABASE.md)
- [Health and readiness](HEALTH_CHECKS.md)
