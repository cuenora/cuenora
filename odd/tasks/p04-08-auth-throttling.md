# P04.08 — Bounded authentication throttling
Branch: `feat/p04-08-auth-throttling`; delivery: exception-ok; forecast: approximately 900 authored changed lines, one coherent security unit.
Runner: pinned Rust 1.99.1 Cargo checks, repository checker and disposable PostgreSQL suite. Execution record: `docs/plans/active/2026-10-27-auth-throttling.md`.

## Specs

- S1: “Comencemos la tarea P04.08 del plan maestro”. Roadmap scope: “Add rate limits and protections to sensitive endpoints.” Existing registration/login/refresh only; preserve previous below-limit behavior, body/concurrency limits, safe errors and financial state. Check route budgets, isolation, expiry, concurrent admission, bounded state, rejection before persistence, and existing tests.
- S2: “Va a estar detras de cloudflare desplegado con dokploy que usa traefik”; “nube naranja apuntando al servidor”. Use an explicitly configured trusted immediate proxy; never trust arbitrary forwarded headers. Check direct/proxy modes, malformed/duplicate/missing headers and untrusted peers. Document origin isolation as an unverified deployment prerequisite, not performed work.
- S3: “Una sola instancia”. Use bounded expiring process-local counters; document reset on restart and no replica coordination. No Redis, database schema changes or remote operations.

- S4: User: “Sí” to “¿Autorizás mantenerlo en una sola PR con excepción de tamaño, sin publicarla todavía?”. One cohesive oversized P04.08 PR is accepted; local work-unit commit is in scope. No publication, remote credential use or native-review consent is granted.

## Tasks

- T1 — S1–S4 — inline implementation, tests and docs — functional implementation/verification complete; RED observed (two HTTP failures), GREEN observed (5 HTTP tests); size exception approved; commit and native review pending. Risk: items 2 and 4 (trusted client identity and synchronized admission). Independent verifier required after own checks. One behavior, no parallel-writer reason.

## Log

- L1: User: “Comencemos la tarea P04.08 del plan maestro”.
- L2: User: “Va a estar detras de cloudflare desplegado con dokploy que usa traefik”.
- L3: User: “nube naranja apuntando al servidor”.
- L4: User: “Una sola instancia”.
- L5: Read-only map and parent spot check on main `81b82a0`: 8 KiB body limits and two password/two refresh permits exist, no temporal throttling or peer metadata. `.atl/` is pre-existing untracked user state. No remote access authorized.
- L6: Implementation defaults (engineering choices, not user-provided quotas): fixed 60-second windows, register 5/IP, login 20/IP and 10/canonical login identifier across IPs, refresh 60/IP; all attempts count. Bounded total state, monotonic expiry, no permanent account lock. Defaults need operational tuning and are not a capacity guarantee. Canonical email uses the existing Email parser and lowercase comparison, with an ephemeral keyed hash rather than retained raw email. Reject before DB/hash work. Return typed 429 plus Retry-After; fail closed with safe 503 if peer identity or limiter state is unavailable. Keep liveness/404/405 outside throttling.
- L7: Trust exact configured immediate Traefik peer IPs only (no implicit private-network trust or new CIDR dependency). In proxy mode require one valid CF-Connecting-IP. Operators must restrict origin HTTP(S) to Cloudflare, isolate API from direct/untrusted internal traffic, maintain trusted peer configuration and avoid header rewriting/Worker/Pseudo IPv4 assumptions. These prerequisites cannot be verified locally.

- L8: RED: initial public HTTP tests failed with 400 instead of 429 for registration and 503 instead of 429 for cross-IP login identifier exhaustion. GREEN: focused HTTP suite subsequently passed 5 tests, with one disposable PostgreSQL test intentionally ignored outside its harness. Formatting normalized. Added transport metadata to existing HTTP fixture routers; no production bypass. Full checks pending. IPv6 client budgets use /64; exact proxy trust does not.

- L9: Own checks (read-only test executor, paths-injected): pinned fmt and Clippy with warnings denied passed; workspace 66 passed/32 intentionally ignored DB cases; repository checker 209 files passed; full disposable PostgreSQL suite 46 passed in 922.2 s. OpenAPI YAML parsed and every local reference resolved. Production behavior unchanged afterward.
- L10: Independent verifier (paths-injected) PASS S1–S3, no blockers: 34 below-budget TCP baseline/candidate cases identical after date/request-ID normalization; real trusted/untrusted peer/header and IPv4mapped/IPv6 grouping, cross-IP identifier, exact concurrent 20/64 admission, window expiry and startup configuration probes passed. Three private units passed; focused disposable PostgreSQL passed (37.9 s), fresh-email strengthened scratch variant passed (15.7 s). No workspace edits by verifier.
- L11: Retained verifier's fresh-email registration probe: a duplicate existing identity might be a no-op if accidentally executed, whereas a new valid email makes forbidden persistence observable. Parent normalized, reran three limiter units and strengthened focused PostgreSQL case (one passed, 15.479 s); no production-code correction was necessary. Full-suite evidence precedes only this supplemental test-data strengthening and passive documentation updates.
- L12: One honest slicing pass: the limiter's trust/admission integration, typed HTTP responses, public-interface regressions and no-write proof are one dependent security behavior. Splitting tests/docs away would weaken review; one cohesive oversized unit is recommended rather than cosmetic reduction. Await explicit P04.08 size exception before any commit; previous feature exceptions do not transfer. No push, PR, credentials or remote operations authorized.
- L13: Review switch explicitly global ON. Preliminary native assess refused undeclared untracked inventory, returned high/unassessable and review_due=true; this is pending proof, not low risk or approval. No lifecycle START/authority was created. After selected commit boundary, perform canonical inventory preflight and exact declared assessment; native candidate consent remains separate from size approval. Preserve .atl/.

- L14: Final local candidate before size decision: 1150 authored changed lines in 21 files (excluding pre-existing .atl/). Runtime denied the parent cleanup command (rm-style commands forbidden), so verifier/test logs remain at /tmp/p0408-verification.yHMiTm/, /tmp/cuenora-p0408-verify.hKsoJF, /tmp/cuenora-p0408-verifier-current and parent focused/spot log paths. Do not claim cleanup or retry through a substitute. All launched processes/Compose resources were already stopped by their owners. All source changes remain unstaged/uncommitted; no publication.

- L15: User: “Sí” to S4. This feature now uses exception-ok for one cohesive oversized PR. Continue local commit and native assessment; separate candidate consent remains required. No push, PR, merge, remote credential use or history rewriting.
