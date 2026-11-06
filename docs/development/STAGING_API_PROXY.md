# Staging API ingress and proxy trust

The 2026-11-06 staging checks support the intended Cloudflare → Traefik → API
chain, private PostgreSQL connectivity and separate authentication budgets for
the two tested clients. **P14.04 is completed for this verified staging scope.**
The evidence is operator-reported; this is not production certification.

Use the [deployment checklist](SETUP.md#cloudflare-proxied-dns--dokploytraefik--api)
and [ADR-020](../architecture/decisions/020-authentication-throttling.md) for the
application's trust contract. This guide records the deployment-specific evidence
and the manual maintenance obligation; it does not install or change anything.

## Observed deployment

| Component | Network boundary |
| --- | --- |
| Standalone Dokploy Traefik | Added to `cuenora-ingress`; its other applications' existing networks were preserved |
| Existing API Application / Swarm service | Only `cuenora-ingress` and `cuenora-data`; no published service port |
| Existing PostgreSQL Swarm service | Only `cuenora-data`; no published service port; original named volume and mount retained |
| `cuenora-ingress` | IPv4 overlay, attachable, not internal; intended application members are Traefik and API |
| `cuenora-data` | IPv4 overlay, internal, not attachable; intended application members are API and PostgreSQL |

The rollout used Dokploy v0.31.0's current Networks selections and
`detachDokployNetwork`, not deprecated Isolated Deployment or a Compose migration.
Both services were pinned to their previously running image digests; no binary
upgrade, schema change or volume replacement was part of the network cutover.
Shared-network membership was removed only after staged private-path checks.

The API's Traefik file-provider configuration retained:

- The Cloudflare-only `ipAllowList` on **both** HTTP and HTTPS routers. On HTTP,
  it precedes the HTTPS redirect. No `ipStrategy` override selects a client-supplied
  forwarded address.
- All 15 IPv4 and 7 IPv6 ranges from the official Cloudflare lists checked on
  2026-11-06. Maintain these against the [IPv4](https://www.cloudflare.com/ips-v4)
  and [IPv6](https://www.cloudflare.com/ips-v6) sources; they are not immutable.
- A task-DNS upstream, `http://tasks.<api-service>:3000`, and
  `passHostHeader: true`. The previous service-VIP path exposed a network endpoint
  as the API's TCP peer; the task path exposed the actual Traefik peer.

`CUENORA_TRUSTED_PROXY_IPS` was enabled only after isolation and observation of
the exact Traefik peer. A subsequent API deployment showed the expected runtime
setting, healthy status, unchanged private network memberships, readiness 200 and
the same proxy peer. Private task addresses are intentionally not copied here:
they can change and must be observed again, not reused from this record.

## Evidence and its provenance — 2026-11-06

The operator executed the server/local-client commands and supplied their output.
The agent inspected that output, a Cloudflare settings screenshot, the final
Traefik YAML and local source. Independent agent review assessed the supplied
evidence and procedure, **not the live server**.

| Check | Observed result | Boundary |
| --- | --- | --- |
| Public readiness across staged API/PG network changes | Clean one-URL checks returned 200; API container reported healthy | Sampled availability, not continuous uptime or complete financial workflows |
| Final public HTTP/TLS smoke from the local computer | HTTP health redirected with 301 to the exact HTTPS URL; readiness, sign-in and recovery pages returned 200 with curl constrained to TLS 1.3 and certificate verification result 0 | Current deployed public paths, not all URLs or automatic renewal |
| Final public security headers | HTTPS responses included HSTS `max-age=31536000; includeSubDomains; preload` and `nosniff`; both identity pages included restrictive CSP and `Referrer-Policy: no-referrer` | Observed headers, not proof of HSTS preload-list registration |
| Final direct-origin HTTP and HTTPS, including forged client-IP headers | All four checks returned 403; HTTPS certificate verification result 0; the bounded script completed | Explicit origin IPv4 via local curl `--resolve`; not external IPv6 or a host-wide firewall audit |
| Origin certificate | Cloudflare Origin CA issuer; direct HTTPS checked using the official Origin CA roots, not `-k` | Origin TLS check, not automatic certificate renewal proof |
| Immediate API peer | API socket inspection matched Traefik's private ingress address, including after the trust-setting deployment | One observed deployment; not a stable-IP guarantee |
| Shared-network reachability | Temporary non-root probe on `dokploy-network` timed out connecting to API ingress TCP 3000 and PG data TCP 5432 | Two sampled destinations; no claim about all interfaces, protocols or Docker-admin access |
| Private positive controls | Traefik → API and API → PG both opened TCP before and after the negative probes; endpoint snapshots unchanged | Avoids mistaking unavailable endpoints for isolation |
| Probe cleanup | Exact newly created container ID removed; no downloaded image, published port, application-data mount or credential | No existing container was removed |
| Public protected login with malformed JSON | 400 `invalid_request` | Confirms passage through protection to input validation; not a successful login |
| Trusted proxy without `CF-Connecting-IP` | 503 `protection_unavailable` | Missing provenance fails closed |
| Untrusted peer with forged CF/XFF/real-IP headers | 503 `protection_unavailable` | Headers alone do not grant proxy trust |
| Corrected client A login window | Requests 1–20: 400 `invalid_request`; request 21: 429 `rate_limited` | One single-instance, 60-second process-local window |
| Client A with two changed `X-Forwarded-For` / `X-Real-IP` values | Both remained 429 `rate_limited` | Those forged values did not grant fresh budgets |
| Independent local-computer client B, run at the A-blocked marker | 400 `invalid_request` | Supports separation for these two clients; no separately verified IPv6 `/64` coverage |
| Client A after another 65-second quiet wait | 400 `invalid_request` | Sampled budget recovery |

The operator reported Full (strict), no covering Worker routes or active
Snippets/custom request-header transforms, and Pseudo IPv4 disabled. A screenshot
confirmed **Remove visitor IP headers** disabled. These settings support the
expected header path, but are not a capture of every request's headers.

An initial quota test stopped after its first forged **`CF-Connecting-IP`** request
returned Cloudflare HTTP 403 / error 1000. A separate bounded diagnostic confirmed
that response; Cloudflare [lists an incoming `CF-Connecting-IP` header as a cause
of error 1000](https://developers.cloudflare.com/support/troubleshooting/http-status-codes/cloudflare-1xxx-errors/error-1000/).
The firewall-event export supplied for diagnosis contained no matching API/login
event. Do not infer a DNS fault from the error title or weaken protection to make
the test pass. The corrected quota test above omitted that incoming header and
completed all 25 authorized requests, including B and recovery.

Malformed-login checks used no credentials and were rejected before database
lookup or persistence. They consumed temporary protection counters. No financial
write, recurrence-generation command, database row hash or data-reset operation
was part of these checks. The existing external recurrence schedule was preserved;
that is not proof of a newly due occurrence being generated after the cutover.

## Manual maintenance after Traefik recreation

**Potential API downtime is an accepted limitation.** The inspected Dokploy
v0.31.0 recreation path does not establish automatic restoration of these modern
custom-network attachments: its reconnect helper selects legacy isolated Compose
resources. See the pinned [network resolver](https://github.com/Dokploy/dokploy/blob/v0.31.0/packages/server/src/services/network.ts)
and [Traefik lifecycle source](https://github.com/Dokploy/dokploy/blob/v0.31.0/packages/server/src/services/settings.ts).

This recovery procedure was source-reviewed; **a global Traefik recreation drill
was not performed**. No provider-console recovery path was available to the
operator. Do not recreate/restart the shared proxy merely to test this guide.

### 1. Inspect before changing anything

Use an explicitly authorized administrative session on the intended server.
Keep private copies of the last approved dynamic YAML, modern network settings,
image pins and database volume mapping. Never paste secrets or full environment
output into a report.

Inspect the existing network and current proxy memberships:

```bash
sudo docker network inspect cuenora-ingress --format 'driver={{.Driver}} attachable={{.Attachable}}'
sudo docker inspect dokploy-traefik --format '{{range $name,$net := .NetworkSettings.Networks}}{{printf "%s %s\n" $name $net.IPAddress}}{{end}}'
```

Stop if the network is missing, the deployment is ambiguous or other expected
memberships changed. Do not guess a replacement subnet, touch another application's
networks, attach Traefik to `cuenora-data`, or recreate a service/volume.

### 2. Restore only the missing ingress membership

Only if the existing overlay is attachable and the inspected Traefik container
is missing `cuenora-ingress`, reconnect that one membership:

```bash
sudo docker network connect cuenora-ingress dokploy-traefik
```

Do not repeat it when already connected. Read back memberships and preserve all
previous ones. This command is not a proxy recreation or a PostgreSQL restart.

### 3. Observe the current peer, then update exact trust

Resolve exactly one current running API task and its current PID; never reuse
pre-deployment IDs/PIDs. Substitute the actual service and public hostname:

```bash
(
API_SERVICE='<api-service>'
API_ORIGIN='https://<api-host>'
API_ID=$(sudo docker ps -q --filter "label=com.docker.swarm.service.name=$API_SERVICE" --filter status=running)
[[ "$API_ID" =~ ^[0-9a-f]+$ ]] || { printf 'Expected one running API task.\n'; exit 1; }
API_PID=$(sudo docker inspect --format '{{.State.Pid}}' "$API_ID")
[[ "$API_PID" =~ ^[1-9][0-9]*$ ]] && curl --noproxy '*' --fail-early --fail --connect-timeout 5 --max-time 10 -sS -o /dev/null -w 'ready=%{http_code}\n' --url "$API_ORIGIN/v1/health/ready" && sudo nsenter --target "$API_PID" --net ss -nt state established '( sport = :3000 )'
)
```

Match the observed peer against Traefik's current `cuenora-ingress` address. If
the connection is too short-lived to appear, absence is inconclusive: do not
guess the peer from a network endpoint, VIP or subnet. Investigate before changing
trust. Readiness 200 alone does not validate authentication protection.

If the verified peer changed, update **only** `CUENORA_TRUSTED_PROXY_IPS` in the
existing Backend environment to the exact address, then deploy that API once in
an accepted maintenance window. Preserve its private network selections, image
pin, other environment values and schedule. Never substitute a CIDR, trust the
whole Docker network, or unset the policy to bypass a failure.

### 4. Verify before resuming normal use

- Re-read the deployed task's expected trust setting without dumping its complete
  environment; re-resolve its PID and check actual peer, network memberships and
  health after the API replacement.
- Read back the dynamic YAML: both routers must retain the Cloudflare allowlist,
  HTTP must check it before redirection, and the task-DNS upstream must remain.
  Domain create/update operations can rewrite that file; normal API redeployment
  is not a substitute for this readback.
- Check public malformed-login validation (400), missing provenance from the
  trusted peer (503), and forged provenance from an untrusted peer (503), without
  credentials. An unexpected response is a stop condition, not permission to
  disable security controls.
- Repeat direct-origin rejection and, when separately authorized, bounded
  independent-client rate checks. Expect curl exit 22 for deliberate HTTP errors;
  use `--fail-with-body` when the safe JSON error code is needed.

If restoration fails, retain the failing evidence and use the last approved
configuration as the rollback reference. Do not reset PostgreSQL, replace its
volume, return API/PG to a shared network, or remove proxy trust as an automatic
fallback. A broader rollback requires an explicit maintenance decision.

## Completion boundary and follow-up

The final public smoke and direct-origin checks completed the verified staging
scope of P14.04 on 2026-11-06. Manual restoration after Traefik recreation remains
an explicitly accepted operational limitation, not an unperformed closing test.

This record does not prove external IPv6 origin isolation
(the inspected host had no global IPv6 address), host/provider firewall coverage,
all alternate routers/ports, continuous availability, certificate renewal,
automatic Traefik recreation recovery, multiple API replicas, or production
readiness. Origin restriction here is application-scoped middleware; UFW and
empty Docker user chains were not hardened globally by this work.

The final direct-origin probe used the actual origin IPv4, preserved Host/SNI
through `--resolve`, and verified HTTPS against temporary official Cloudflare
Origin CA roots without disabling certificate verification. Normal and forged
CF/XFF/real-IP requests were rejected over both HTTP and HTTPS. Repeat these
checks after future relevant deployment changes rather than assuming this dated
evidence remains current.

Duplicate/malformed CF-header runtime cases and IPv6 grouping were not separately
exercised in these staging probes. These and the broader operational limitations
above are disclosed coverage boundaries, not additional blockers retroactively
added to the agreed staging task. P14.03 has its own
[restricted-role completion evidence](POSTGRES_RUNTIME_ROLE.md#staging-completion-follow-up--2026-11-06);
production validation remains separate.
