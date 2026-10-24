# Execution plan: Initial identity strategy

- **Status:** Active — direction documented and verified; PR open, awaiting review and merge
- **Date:** 2026-10-24
- **Roadmap task ID(s):** P04.01; prerequisite clarification for P03.04
- **Tracking issue / PR:** [PR #26](https://github.com/cuenora/cuenora/pull/26)
- **Owner:** Maintainer and coding agent

## Goal and user-visible outcome

Document the approved identity direction before designing minimal user,
credential and session storage. Define who owns authentication and the
registration/verification gate without claiming an implemented login system.

## Existing state and evidence

- The roadmap leaves first-party credentials versus OIDC open in P04.01.
- The maintainer chose "Credenciales propias", then "Abierto", and answered
  "Sí" to requiring verified email before financial accounts or movements.
- CodeGraph found no authentication, identity, login, session or migration code.
- P03.03 date documentation is already prepared locally and must be preserved.

## In scope

An identity decision document, ADR, roadmap references and explicit P03.04 design
prerequisites. Keep this documentation unit distinct from runtime implementation.

## Out of scope

SQL migrations, new dependencies, authentication endpoints, email delivery,
password hashing implementation and session/verification/recovery protocols.

## Constraints and invariants

Preserve authenticated ownership boundaries, exact money and date semantics.
Never store or log plaintext passwords, raw verification/recovery/session tokens
or real financial records. Verification does not replace authentication or ownership.

## Steps

- [x] 1. Inspect the roadmap and obtain the three product decisions individually.
- [x] 2. Record the direction and remaining technical decisions before migrations.
- [x] 3. Read back the documents, validate links and run repository checks.

## Acceptance criteria

- [x] First-party email/password, open registration and mandatory verification are explicit.
- [x] Stable internal user ownership is independent of the login identifier.
- [x] Storage prerequisites are distinguished from implemented tables and endpoints.
- [x] P04.01 is checked only after documentation verification; P03.04 stays pending.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Relative Markdown links and structural readback | All local targets and table structures valid | Passed across ten current documents; six identity scenarios specified | Passed |
| `python3 scripts/check_repository.py` | Repository text validation passes | Passed; 102 Git-visible files examined | Passed |
| `git diff --check` | No whitespace errors | Passed | Passed |
| Runtime RED/GREEN | Not applicable to a passive decision | No source, dependencies or schema changed | Not applicable |

## Risks, assumptions, and decisions

See [identity strategy](../../product/identity-and-access.md) and
[ADR-004](../../architecture/decisions/004-first-party-identity.md). Security-sensitive
implementation still requires its own tests and verification; this document grants
no authentication assurance or production-readiness claim.

## Completion notes

Prepared inline with the existing P03.03 documents on `docs/date-and-identity-semantics`.
PR publication was requested on 2026-10-24. Keep this plan active until its delivery
is merged; P03.04 and all authentication implementation tasks remain pending.
Risk: none for current runtime behavior; documentation only.

No authentication, migration, Rust or PostgreSQL runtime checks were run for this
passive documentation task. Future acceptance scenarios remain unimplemented.
No failures remain open.

Published in [PR #26](https://github.com/cuenora/cuenora/pull/26) against `main`.
The maintainer explicitly accepted `size:exception` on 2026-10-24 for the combined
date/identity definition (initial commit `f5237b8`, 537 changed lines).
Authentication implementation and merge are not included in this publication task.
