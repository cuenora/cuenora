# Web login and email verification
Objective: connect first-party browser identity flows (P04.03/P04.12).
Branch: feat/web-login-verification; delivery: exception-ok; forecast: 600 authored lines; runner: Cargo and deterministic browser-script tests.

## Specs
- S1: "Implementemos la verificacion de email y el login web" — connect existing same-origin pages and APIs, including verification mail links; preserve API authentication and explicit confirmation.
- S2: User answered "Sí" to "¿Te parece empezar con la sesión solo en memoria —sin guardar tokens en el navegador, pero obligando a iniciar sesión al recargar— y mostrar una confirmación al ingresar, ya que todavía no existe un dashboard web?" — memory-only credentials and signed-in confirmation; no dashboard.

## Tasks
- T1 [S1,S2] delegated, verified: browser login/verification and tests; parallel disjoint browser unit; Risk item 2 (credentials).
- T2 [S1] delegated, verified: optional trusted verification mail origin and tests/docs; parallel disjoint mail unit; Risk item 2 (challenge links).
- T3 [S1,S2] inline, verified: integration, independent security verification, documentation and work-unit commits after delivery strategy decision.

## Log
- L1: Implementemos la verificacion de email y el login web
- L2: Sí (approved memory-only session and signed-in confirmation without dashboard).
- Evidence: existing verification requires bearer identity; recovery is fragment/in-memory precedent. RDD globally disabled. Preserve existing .atl and Python caches.
- L3: Sí (approved one PR with size exception for this feature).
- L4: T1/S1,S2 passed Node 14/14 and HTTP 3/3; T2/S1 mail 18/18 and config 12/12. RED observed missing implementation; parent regression RED observed transient login losing challenge, then GREEN.
- L5: Workspace seam 177 passed, 245 ignored PostgreSQL cases; fmt, Clippy, repository validation and diff checks passed. Recovery Node 15/15. No live provider/real-browser functional/database harness execution. Independent verifier passed S1/S2 after one correction/recheck for challenge preservation. RDD disabled/unmanaged; assessment unavailable due to ambient untracked inventory, no review started.
- L6: Single PR size exception approved; one cohesive identity work unit includes both browser and optional mail links. Publication not performed.
