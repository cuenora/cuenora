# Configure process health and readiness

Use **liveness to detect a stopped/broken process**, and **readiness to decide
whether the API can currently serve database-dependent traffic**. These endpoints
are unauthenticated, return no dependency details and do not migrate the database.

| Check | Success | Failure | Purpose |
| --- | --- | --- | --- |
| `GET /v1/health/live` | 200, exact `OK` | Process/network failure | Process health; does not consult PostgreSQL |
| `GET /v1/health/ready` | 200, exact `OK` | 503, exact `Unavailable` | DB connection and required read-only schema availability |

Readiness has `Cache-Control: no-store` and a one-second total deadline; its read-only transaction uses 750ms local statement/lock timeouts. Missing
DB configuration, errors, absent required schema or an already-running readiness
probe fail closed. Probes do not queue and close their DB connection on completion
or cancellation. Health HTTP requests require no credentials and responses expose no SQL diagnostics.

The schema probe resolves 25 current public tables and selected compatibility
columns with empty SELECTs. It is not a migration-history/checksum check or proof
of all constraints/types/write permissions, mail delivery, scheduler health or
complete application readiness. It does not inspect financial values.

## Container probe

New images include:

```sh
cuenora-healthcheck
cuenora-healthcheck --help
```

The command reads only `CUENORA_LISTEN_ADDR`, defaulting to `127.0.0.1:3000`.
Wildcard binds map to loopback, including IPv6. It checks local HTTP liveness,
requires 200 and exact `OK`, rejects redirects, ignores proxy/dependency settings,
and has a two-second request/body deadline. Success exits 0 without output;
failure exits nonzero with `Health check failed.` and no raw diagnostics.

Docker HEALTHCHECK runs it every 30 seconds, with a three-second Docker timeout,
ten-second start period and three retries. The image remains UID/GID 65532.
A Docker `unhealthy` status does not itself establish a restart/traffic policy;
the orchestrator owns that behavior. Alpha.2 predates this feature; alpha.5 includes
it and was deployed to staging after PR #111 merged. See the completed record below.

## Dokploy rollout checklist

The alpha.5 deployment has operator-reported Docker health and public endpoint
verification, including the PostgreSQL stop/start readiness cycle. Exact override
settings and readiness-only traffic gating were not inspected; the checklist below
is not a claim that every deployment boundary has been audited.

1. Keep `CUENORA_LISTEN_ADDR=0.0.0.0:3000` for container networking and expose the
   service only through the intended private proxy network.
2. Deploy the reviewed image digest, then inspect its container health. If the
   Dokploy service type does not use Dockerfile HEALTHCHECK automatically, configure
   its process-health command as `cuenora-healthcheck` with the timings above.
3. Configure HTTP readiness/traffic admission against `/v1/health/ready` on the
   internal port 3000 only if the deployed platform supports a separate traffic
   probe. Do not substitute it for liveness or assume a generic health setting
   separates restarts from routing. If unsupported, use it for monitoring and
   explicit deployment checks until traffic gating is configured deliberately.
4. Check both endpoints after migrations. During an isolated, approved database
   interruption, verify live remains 200 while ready becomes 503, and recovery
   restores ready. Never run that experiment against real/personal data.
5. Preserve origin ingress restrictions, trusted immediate proxy configuration and
   HTTPS. Public endpoint success does not prove those boundaries.

### Dokploy Application / Swarm health setting

Dokploy documents this at **Advanced → Cluster Settings → Swarm Settings → Health
Check**. That check can affect restarts/rollbacks, so use the liveness executable,
not the readiness URL. Timings below are nanoseconds, matching the Docker API:

```json
{
  "Test": ["CMD", "cuenora-healthcheck"],
  "Interval": 30000000000,
  "Timeout": 3000000000,
  "StartPeriod": 10000000000,
  "Retries": 3
}
```

Save and redeploy only after selecting the reviewed new image. This is a proposed
configuration, not confirmation that the live service uses these exact overrides. The documented
Swarm health setting is not a separate readiness-only traffic gate. See
[Dokploy advanced settings](https://docs.dokploy.com/docs/core/applications/advanced)
and [health checks/rollbacks](https://docs.dokploy.com/docs/core/applications/going-production).
Plain Docker health status and restart policy also differ; see
[Docker restart policies](https://docs.docker.com/engine/containers/start-containers-automatically/).

## Local verification

```bash
cd backend
cargo test -p cuenora-api --test readiness --test healthcheck --locked
cd ..
python3 tests/infrastructure/test_postgres_compose.py PostgresComposeTest.test_readiness_against_disposable_postgres -v
python3 tests/infrastructure/test_api_docker.py
```

Ordinary Cargo ignores six database tests. The explicit PostgreSQL harness uses
only a fresh isolated local Docker database. No staging credentials are needed.
See [ADR-047](../architecture/decisions/047-process-health-and-schema-readiness.md)
and the [completed execution record](../plans/completed/2026-11-04-health-readiness.md).
