# Execution plan: Validated runtime environment

- **Status:** Completed
- **Date:** 2026-11-02
- **Roadmap:** Operational configuration supporting P04/P14; no phase completion implied.

## Authorized scope

Keep current defaults, reject invalid settings without secrets, document every setting
in `.env.example`, read process environment only (no dotenv), and verify Docker.
Settings: listen address, public API URL for rendered OpenAPI, token lifetimes,
PostgreSQL pool capacity/acquisition timeout and statement/lock timeouts.
Preserve existing ownership/transfer verification edits and `.atl/`.

## Implementation and verification

- [x] Parse immutable configuration once with bounded numeric values and safe errors.
- [x] Inject session lifetimes consistently into persistence and response paths.
- [x] Centralize database timeouts without transaction-local overrides.
- [x] Render deployment OpenAPI without mutating the source contract.
- [x] Add env examples, Docker execution and configuration regression tests.
- [x] Run Cargo, isolated PostgreSQL and Docker checks; independent security review.

## Decisions and constraints

No dotenv dependency. Existing constructors retain defaults for compatibility.
No financial rules, hash costs, throttles or body limits become environment settings.
No remote deployment, publishing or unrelated source changes. Docker is local only.
Tests cover defaults, valid overrides and rejected inputs without echoing values.

## Observed outcome

Implementation completed locally; no commit, push or PR. All checklist outcomes
above were observed. No roadmap phase is declared complete by this change.

| Check | Observed result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-features --locked` | Passed; ordinary Cargo ignores PostgreSQL cases |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Passed |
| Disposable PostgreSQL: login, refresh, mutations, transfers, recurring rules, recurrence job, flows | Six targets passed initial run; login passed after fixing test transport metadata (6 tests, 23.555s). Configured TTLs match response and stored expiry for initial and rotated tokens; configured session timeouts checked. |
| Docker API runtime test | Passed against final corrected candidate; nonroot, default/override bind, liveness, environment, renderer, invalid settings/redaction |
| OpenAPI structural tests | 7 passed |
| Repository validation / whitespace | Passed |
| Independent verification | Malformed public URL authorities were accepted by HTTP URI parsing; fixed with explicit host, IPv6 and port validation. Regression observed RED then GREEN. Targeted recheck: 27 rejected inputs across all 3 binaries and 6 valid controls passed. |

The initial Docker assertion compared YAML quotation literally; fixed the test to
decode the renderer's quoted scalar. The new login test initially lacked TCP
metadata required by the auth protections; corrected the fixture, not the runtime.
Initial Clippy reported a nested-if style error, corrected without suppression.
These failed intermediate checks are not counted as passing runs.

Risk: items 2, 3 and 5 — configurable session policy, TTL contract ranges,
and runtime/container settings. Independent security/config verification passed
after one authority-validation correction. Native RDD remains globally disabled;
no review consent, remote deployment or publication was performed.

## Archive reconciliation — 2026-11-04

Acceptance and recorded scoped verification are complete. Integration is present
in local `main`: `d43727b feat(config): validate runtime environment and verify resource isolation (#104)`. Earlier statements that merge or
publication was pending describe the pre-merge state and are superseded by this
reconciliation. Archiving does not assert production deployment or rerun tests.
