# Architecture decisions

ADRs record chosen direction and its tradeoffs; acceptance does not mean every
component has been implemented. Implementation status is dated within each record.

| ADR | Decision | Status |
| --- | --- | --- |
| [001](001-initial-stack.md) | Initial backend, mobile and delivery stack | Accepted |
| [002](002-exact-money.md) | Exact recorded money, bounds and quantization | Accepted; domain types implemented |
| [003](003-date-semantics.md) | Business dates, UTC audit instants and stable named zones | Accepted design; runtime implementation pending |
| [004](004-first-party-identity.md) | First-party credentials, open registration and email verification | Accepted direction; not implemented |
| [005](005-identity-storage.md) | Case-insensitive ASCII/IDNA identity and opaque refresh-session storage | Schema implemented; authentication runtime pending |
| [006](006-append-only-account-movements.md) | Append-only account postings and compensating corrections | Accepted; storage implemented, correction workflow pending |
| [007](007-minimal-transaction-audit.md) | Field-name-only append-only transaction audit | Accepted; storage implemented, automatic auditing pending |
| [008](008-account-reference-compatibility.md) | Account type/currency changes preserve existing financial references | Accepted; native storage constraints |
| [009](009-registration-credentials.md) | Registration input policy and salted Argon2id credentials | Credential primitives; endpoint pending |
| [010](010-atomic-registration-storage.md) | Atomic pending registration without duplicate credential replacement | Persistence adapter; HTTP pending |

Add a new numbered ADR when changing an accepted decision. Link the superseded
record rather than erasing its rationale. Roadmap tasks remain in the
[master plan](../../plans/master-plan.md).
