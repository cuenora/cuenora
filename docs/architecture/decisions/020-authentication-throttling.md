# ADR-020: Bounded process-local authentication throttling

- **Status:** Implemented and verified locally; commit/native review pending
- **Date:** 2026-10-27
- **Roadmap:** P04.08

## Decision

Use one shared in-process limiter for the maintainer-confirmed single API
instance. No Redis, new dependency or schema change is required. Fixed windows
start at the first admitted attempt and expire after 60 monotonic seconds:

| Key | Attempts per window |
| --- | --- |
| Registration client IP | 5 |
| Login client IP | 20 |
| Refresh client IP | 60 |
| Canonical login email, across client IPs | 10 |

These are conservative implementation defaults, not measured capacity or
maintainer-specified quotas. All attempts count, including successful requests;
IP admission precedes body parsing and email admission precedes password/DB work.
The existing email parser and case-insensitive comparison define the login key;
retain only a process-keyed hash, not raw email. Do not reset on success or use
permanent account locks. A targeted attacker can still temporarily consume a
known identifier's budget; recovery/MFA and adaptive controls remain future work.

Keep at most 8,192 live key slots. Expired entries are reclaimed with a bounded
scan at most once per second. Never evict an active entry to admit a new identity.
A full/poisoned store fails closed with a generic 503 rather than bypassing limits.
A short mutex protects only in-memory admission; no body, hash or DB await occurs
while held. Router clones share counters; separate app constructions do not.
IPv4-mapped IPv6 is canonicalized to IPv4, and IPv6 client keys group by /64.

Return `429 rate_limited`, a positive integer `Retry-After` (seconds until the
window expires, rounded up), `Cache-Control: no-store`, and the existing typed
request-ID envelope. Do not disclose which limiter fired. Missing/untrusted peer
identity fails closed with `503 protection_unavailable`. Health, unknown paths
and unsupported methods are not throttled. Preserve body and semaphore limits.

## Trust boundary

Direct mode (default) uses the TCP peer and ignores every forwarded IP header.
Proxy mode requires `CUENORA_TRUSTED_PROXY_IPS`: a nonempty comma-separated list
of at most 32 exact IPv4/IPv6 immediate Traefik peer addresses. No CIDR, wildcard,
automatic Docker-range trust or insecure fallback. Malformed configuration fails
startup with a fixed safe message. Require exactly one valid `CF-Connecting-IP`
from a configured immediate peer, otherwise reject authentication requests.
`X-Forwarded-For`, `X-Real-IP` and the mere presence of a Cloudflare header do not
authenticate the chain. Missing socket metadata also fails closed.

The operator must ensure the public origin accepts HTTP(S) only from Cloudflare,
Traefik forwards the header unchanged, and only that proxy can reach the API.
Trusting a proxy IP alone does not prove Cloudflare was upstream. Do not expose
this mode on an unrestricted origin or shared untrusted container network.
Exact peer addresses require stable networking and updates after topology changes.
Cloudflare Workers/header transforms and Pseudo IPv4 overwrite mode require
separate validation; this implementation assumes ordinary proxied DNS without
those modifications. No live topology or firewall was inspected or changed.

## Tradeoffs and alternatives

Memory state is bounded and adds no storage service, but resets on restart and
cannot coordinate replicas. Fixed windows permit a boundary burst. Shared NATs
share IP budgets; /64 grouping trades IPv6 rotation resistance for shared-network
fairness. A global bounded store may reject legitimate newcomers under attack;
Cloudflare/Traefik edge protections are still necessary for volumetric abuse.

A shared external store would survive process replacement and coordinate replicas,
but adds operational/failure and atomicity requirements. An edge-only limiter
would avoid application state but cannot consistently implement the typed API
contract or canonical login-identifier control. Revisit before adding replicas.

## Sources

- [Cloudflare HTTP headers](https://developers.cloudflare.com/fundamentals/reference/http-headers/)
- [Cloudflare origin restrictions](https://developers.cloudflare.com/fundamentals/concepts/cloudflare-ip-addresses/)
- [Traefik forwarded-header trust](https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/)
- [OWASP login throttling and lockout tradeoffs](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html#login-throttling)

## Verification

Own pinned format/Clippy/workspace checks passed (66 tests, 32 deliberately
ignored DB cases); all 46 disposable PostgreSQL infrastructure cases passed.
Independent verification passed 34 below-budget TCP differential cases against
`81b82a0`, real trusted/untrusted peer, concurrency and expiry probes, plus focused
PostgreSQL proof. Parent repeated the three limiter units and strengthened
fresh-email registration rejection case. Origin deployment remains unverified.
See the [active execution record](../../plans/active/2026-10-27-auth-throttling.md)
for exact evidence and remaining delivery steps.
