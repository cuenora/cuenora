# ADR-004: First-party credentials with open, verified registration

- **Status:** Accepted direction; not implemented
- **Date:** 2026-10-24
- **Scope:** P04.01; prerequisite direction for P03.04

## Context

The roadmap leaves identity ownership open while P03.04 needs minimal user and
session/identity storage. The maintainer selected first-party credentials,
open registration and email verification before financial account creation or
recording movements. No authentication or persistence implementation exists.

## Decision

Start with Cuenora-managed email/password authentication. Permit self-service
registration without invitations. A registration remains pending verification;
the server must enforce verification before enabling financial operations.
Retain authenticated ownership checks independently of email verification.

Use stable internal user identifiers as financial ownership keys. Keep credential
identifiers separate from those keys and follow the roadmap's Argon2id direction.
The [identity strategy](../../product/identity-and-access.md) records the product
decisions, required controls, acceptance boundaries and unresolved storage details.

Do not select credential/session columns by assumption. Email comparison,
session/token format, challenge lifecycle, expiration values, hashing parameters
and email transport still need explicit implementation decisions and tests.
No library, service, schema, security control or runtime endpoint is introduced here.

## Alternatives and tradeoffs

| Alternative | Benefit | Tradeoff behind this choice |
| --- | --- | --- |
| External or self-hosted OIDC provider | Delegates credential authentication to an identity service | Adds a provider integration and operational dependency; the maintainer chose first-party credentials. |
| Invitation-only registration | Controls who may create identities | The maintainer explicitly chose open self-service registration. |
| Financial access before email verification | Fewer initial steps | Rejected by the maintainer; financial use requires verified email ownership. |

## Consequences and status

Cuenora owns password verification, email challenges, recovery and session
security. Open registration requires appropriate bounded/abuse-resistant flows
when implemented. Documenting this direction completes P04.01 only; P03.04 and
the remaining authentication tasks are not complete.

Revisit through a new ADR if login ownership or the registration policy changes.
Adding another login mechanism later must preserve existing internal ownership
and must not merge identities based on an unverified email match.

**2026-10-24 follow-up:** [ADR-005](005-identity-storage.md) resolves the P03.04
email and refresh-session storage choices. Its migrations do not implement the
authentication runtime described here.
