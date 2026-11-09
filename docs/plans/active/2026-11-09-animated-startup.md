# Execution plan: Animated session-ready splash

- **Status:** Active
- **Date:** 2026-11-09
- **Roadmap task ID(s):** P11.01/P11.02 startup follow-up
- **Tracking issue / PR:** Creation authorized; two independent PRs target main
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Show a blue C without the disc, animate its three curves, and keep splash visible until the full animation and startup verification complete.

## Existing state and evidence
Native startup exposes sequential auth and home/profile loaders. Existing splash assets are uncommitted; user wants them revised before publication. See [task specification](../../../odd/tasks/android-animated-startup.md).

## In scope
Native vector animation, static/disabled-animation fallback, compat splash lifecycle, auth+initial home readiness observation and tests.

## Out of scope
New authentication semantics, financial writes, loading-screen removal, launcher icon changes, extra splash Activity, remote publication.

## Constraints and invariants
Preserve existing loaders and errors behind splash. Auth storage/profile failures release to existing recovery UI. No auto-post/replay or owner changes. Keep minSdk24 and existing navigation/data policies. Observe native animation timing, including disabled/scaled animations, without shortening the complete logo.

## Steps
- [x] Animation resources and rendered regressions.
- [x] Startup readiness/exit integration and native tests.
- [x] Seam suites, independent verification and docs.

## Acceptance criteria
- [x] S1–S4 verified.
- [x] Launcher unchanged; complete mark under supported masks.
- [x] Fast/slow auth, signed-out/in, failures, reduced motion and lifecycle disposal covered.

## Verification
| Check | Expected | Observed |
| --- | --- | --- |
| Resource checks and app build/lint | Pass | Python5, build and lint0 passed |
| Isolated API37/native compatibility checks | Pass | Focused10/full88 passed; final debug-host cleanup rebuild/focused10 passed. Pre31 source-inspected only |
| Independent startup timing/authority probes | Pass | S1–S4 no blockers,145 controller combinations/probes and baseline invariants passed |

## Risks, assumptions, and decisions
Risk items2/4/5: session presentation, async lifecycle and a new pinned AndroidX compat dependency. No authority/state-machine changes intended. A full animation may intentionally delay fast startup by its remaining duration, as explicitly requested.

## Completion notes
Local implementation verified; physical installation is pending because the authorized Samsung is disconnected. Implementation commit `5d4c57a`; publication authorized with a size exception, no merge authorized. The maintainer explicitly authorizes installing the finished, verified new app on the physical Samsung; no test runner, financial operation or backend mutation is authorized on that device.

Standalone `main`-based branch verification passed: app/test build, lint0, full native52 and resource2. Independent source/boundary checks passed, including both conflict-free merge orders with the transaction branch. Pre31 runtime and physical-device follow-up remain explicitly pending.
