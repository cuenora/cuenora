# Execution plan: Multi-platform backend alpha images

- **Status:** Active
- **Date:** 2026-11-04
- **Roadmap task ID(s):** P14.02 (publication support, not production readiness)
- **Tracking issue / PR:** Not created

## Goal and user-visible outcome

Publish a tested backend alpha image to `carfdev/cuenora` for both
`linux/amd64` and `linux/arm64`. The maintainer reports that Docker Hub and
GitHub Actions credentials are configured; secret values must never be read.

## Existing state and evidence

`infra/Dockerfile.api` builds API/OpenAPI executables and runs as UID 65532.
The existing Docker harness checks liveness, configuration, redaction and OpenAPI.
No publishing workflow exists. The image does not deploy PostgreSQL or migrations
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

- [ ] Native amd64/arm64 images pass the existing runtime harness.
- [x] Workflow gates publication on both builds and verifies both manifest platforms; hosted execution remains pending.
- [x] Operator instructions, YAML/shell/Python syntax and local structural checks pass.
- [x] Independent Risk item 5 verification passed with no blockers.
- [ ] Hosted build/publication and registry evidence are recorded before claiming release.

## Verification

| Check | Observed result |
| --- | --- |
| Repository validation and whitespace | Passed locally |
| Workflow YAML/permissions/matrix/gates and shell/inline Python parsing | Passed locally |
| Publication shell with fake Docker | Two test groups passed: both digest uploads precede final manifest; either failed upload prevents final tag creation |
| Existing Docker runtime harness | Passed locally; not evidence of native ARM64 |
| Independent scratch verification | Passed: regression tests, gates/credential isolation, shell/Python parsing, architecture/digest/manifest failure probes; parent reran regression tests |
| Native hosted AMD64/ARM64 builds and Docker Hub manifest | Pending |

The workflow itself has no local GitHub Actions runner for a meaningful RED/GREEN
publication test. Its new shell regression checks use a fake Docker command and
never credentials or registry mutations. Existing runtime behavior is unchanged.

## Risks, assumptions, and decisions

Risk: item 5 (CI and container publication). GitHub native ARM runners avoid Rust
emulation. Alpha tags use the workflow run number; deploy by manifest digest for
reproducibility. Base image tags are not immutable, so reruns may rebuild differently.

## Completion notes

Local implementation and verification complete; hosted builds/publication pending.
Integrated the work onto local main `907e40b` after PR #108 merged. Repository,
regression and workflow structure/syntax checks passed again after integration.
The implementation PR validates native builds but cannot publish on pull requests.
Merge and explicit manual dispatch on main remain necessary for publication.
