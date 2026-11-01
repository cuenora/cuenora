# P07.09 — Safe daily recurrence job
Objective: one-shot generation job with cross-process exclusion and bounded failure handling. Branch: `feat/p07-09-daily-job`; delivery baseline: `0fcb7a9` (merged PR91, byte-identical to original test baseline `48253a7`).
Delivery: exception-ok; maintainer authorized one PR with size:exception, no issue, current gh session and SSH push. Runner: Rust workspace and disposable PostgreSQL harness.

## Specs
- S1: "Mientras corre el CI de esta PR vamos avanzando con la tarea siguiente P07.09" — "Add a safe daily worker/job with concurrency control." Process due occurrences using existing P07.08 date, pause, end, snapshot and no-posting invariants.
- S2: Maintainer answered "Sí" to "¿Confirmás este enfoque para implementarlo en una rama nueva basada en P07.08, sin modificar la PR #91?" The proposed approach was "comando de una sola pasada", "programado diariamente mediante cron o scheduler", "con bloqueo PostgreSQL para evitar ejecuciones solapadas". No embedded HTTP worker or scheduler installation.
- S3: Preserve existing bearer/public generation behavior. Discover verified-owner active rules with bounded keyset pages; use database-derived ownership and revalidate under owner → rule locks. Isolate rule failures and signal partial failure; no financial/user details in output.
- S4: Hold advisory exclusion on one close-on-drop database session for both discovery and per-rule transactions. Overlap is a successful no-op; error/cancellation cannot leak a pooled lock or leave partial per-rule data. Reuse CUENORA_DATABASE_URL; safe help/config/argument/exit behavior, preserving existing cargo run API selection.

## Tasks
- T1 | S1–S4 | inline (one coherent job behavior; no writer reason) | completed: CLI + 5 original DB groups RED → GREEN; final job 10/10 and CLI 2/2 | commit: c9a4898eeb9aea8d6dd0a6c9cc601dd8be56a4a5
- T2 | S1–S4 | delegated verification (risk items 2/3/4: shared owner authorization, existing generator and concurrency) | completed: independent S1–S4 PASS | commit: c9a4898eeb9aea8d6dd0a6c9cc601dd8be56a4a5

## Log
- L1: "Mientras corre el CI de esta PR vamos avanzando con la tarea siguiente P07.09".
- L2: User answered "Sí" to the quoted S2 proposal. PR91 is not to be modified; branch begins at its head. Only preexisting untracked `.atl/`; preserve it. RDD globally off.
- L3: Keep advisory lock and actual generation on the same physical session: losing that session must remove write capability as well as release exclusion. SQLx 0.8.6 close_on_drop verified in installed source. Reuse transactions/ownership, never synthesize AuthenticatedIdentity.
- L4: Planned defaults: 100-row keyset pages bounded by an initial high watermark; one transaction per rule; process other rules after a failure, return nonzero if any fail. One-shot CLI help exits 0, invalid args/config 2, job failure 1, overlapping invocation 0. External daily schedule is deployment work, not installed by this task.

- L5: Own verification passed fmt/Clippy/workspace; final Cargo 108 passed/0 failed/155 ignored. Three applicable isolated PostgreSQL targets passed: initial job5, generation10 and lifecycle9. Retained verifier probes expand final job target to10; final focused job10 and two CLI groups passed. Full unrelated infrastructure suite not rerun. Cargo metadata confirms unchanged API default.
- L6: Independent S1–S4 PASS: backend termination removes generation capability, owner verification/pause changes after discovery revalidate, dropped sessions are not pooled, cancellation of unknown advisory result releases exclusion, deterministic failed-first/healthy-second transaction recovery. Baseline/candidate generation10 and lifecycle9 each pass; 80 normalized HTTP bodies/statuses identical. Parent repeated job10 and unavailable CLI probe. Test-only vector lint corrected before final checks.
- L7: Production source unchanged after full applicable checks; only retained regression probes and documentation followed. PR91 remains unchanged. No scheduler installed or remote operations performed. One bounded slicing pass: helper extraction alone leaves worker/tests/contract over budget, so recommend one coherent local size-exception unit; approval pending before commit.

- L8: User request: "Ya mergee la PR 91, sincroniza main y luego crea otra PR para estos cambios de la tarea P07.09". Verified PR91 MERGED at `0fcb7a936f40aa21d6e4dbc11e8839585a2b583c` and fast-forwarded local main. Its tree matches original test baseline `48253a7` exactly. Preserved all local work on fresh branch `feat/p07-09-daily-job` from main; old unpublished planning branch retained without rewriting it.
- L9: User answered "Si" to "¿Autorizás publicarlos como una única PR con `size:exception`, sin issue y usando la misma sesión de `gh` y SSH?" Publication approved; no merge authorized. `.atl/` remains excluded.

- L10: S1–S4 implementation and all retained probes committed in `c9a4898eeb9aea8d6dd0a6c9cc601dd8be56a4a5`. Delivery slice is this commit plus a documentation-only evidence commit, based solely on merged PR91/main. Native review remains disabled/unmanaged.
