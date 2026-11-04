# Execution plan: Multi-platform backend alpha images

- **Status:** Completed
- **Date:** 2026-11-04
- **Roadmap task ID(s):** P14.02 (publication support, not production readiness)
- **Tracking issue / PR:** https://github.com/cuenora/cuenora/pull/109

## Goal and user-visible outcome

Publish a tested backend alpha image to `carfdev/cuenora` for both
`linux/amd64` and `linux/arm64`. The maintainer reports that Docker Hub and
GitHub Actions credentials are configured; secret values must never be read.

## Existing state and evidence

`infra/Dockerfile.api` builds API/OpenAPI executables and runs as UID 65532.
The existing Docker harness checks liveness, configuration, redaction and OpenAPI.
At plan creation no publishing workflow existed; PR #109 now supplies it. The image does not deploy PostgreSQL or migrations
and does not include the recurrence job binary.

## In scope

Native architecture builds, existing runtime checks, a gated manual publication
workflow, final manifest verification and operator instructions.

## Out of scope

Production deployment, database/migration packaging, mobile, runtime behavior
changes and production-readiness claims.

## Constraints and invariants

Build/test jobs receive no Docker Hub credentials. Publication uses only the
configured Actions token, after both native jobs pass, from main on manual dispatch.
No latest tag. Run-scoped intermediate tags and one versioned multi-platform alpha.

## Steps and acceptance criteria

- [x] Native amd64/arm64 images passed the runtime harness in PR and publication runs.
- [x] Workflow gates publication on both builds and verifies both manifest platforms; hosted execution passed.
- [x] Operator instructions, YAML/shell/Python syntax and local structural checks pass.
- [x] Independent Risk item 5 verification passed with no blockers.
- [x] Hosted publication succeeded and the two-platform Docker Hub manifest was independently inspected.

## Verification

| Check | Observed result |
| --- | --- |
| Repository validation and whitespace | Passed locally |
| Workflow YAML/permissions/matrix/gates and shell/inline Python parsing | Passed locally |
| Publication shell with fake Docker | Two test groups passed: both digest uploads precede final manifest; either failed upload prevents final tag creation |
| Existing Docker runtime harness | Passed locally; not evidence of native ARM64 |
| Independent scratch verification | Passed: regression tests, gates/credential isolation, shell/Python parsing, architecture/digest/manifest failure probes; parent reran regression tests |
| Native hosted AMD64/ARM64 builds and Docker Hub manifest | Run 41831355162 succeeded; alpha.2 has exactly linux/amd64 and linux/arm64 |

The workflow itself has no local GitHub Actions runner for a meaningful RED/GREEN
publication test. Its new shell regression checks use a fake Docker command and
never credentials or registry mutations. Existing runtime behavior is unchanged.

## Risks, assumptions, and decisions

Risk: item 5 (CI and container publication). GitHub native ARM runners avoid Rust
emulation. Alpha tags use the workflow run number; deploy by manifest digest for
reproducibility. Base image tags are not immutable, so reruns may rebuild differently.

## Completion notes

PR #109 merged as `51adc51d4b76ec008d6c6b694273dc6926d842f6`. Manual main dispatch
[41831355162](https://github.com/cuenora/cuenora/actions/runs/41831355162) succeeded:
AMD64 build/runtime checks in 1m51s, ARM64 in 1m37s, publication in 38s.
`carfdev/cuenora:0.1.0-alpha.2` was independently inspected anonymously and contains
exactly `linux/amd64` and `linux/arm64`.

Manifest digest: `sha256:f1c93f8cd299c014b46056b876b54925bec722854ebedec28ee71b1fe4528a47`.

This closes this publication plan, not all of P14.02: non-root/native image checks
are verified, but an ongoing Docker/orchestrator health probe remains unverified.
No production deployment, migration runner or recurrence-job image is delivered.
Archived on 2026-11-04 after merge and publication evidence were verified.
