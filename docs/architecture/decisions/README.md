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
| [009](009-registration-credentials.md) | Registration input policy and salted Argon2id credentials | Implemented registration prerequisite; login pending |
| [010](010-atomic-registration-storage.md) | Atomic pending registration without duplicate credential replacement | Implemented; called by registration HTTP |
| [011](011-bounded-registration-http.md) | Bounded local registration HTTP and explicit database configuration | Implemented and independently verified |
| [012](012-login-credential-primitives.md) | Bounded verification and initial opaque login credentials | Primitives and HTTP login implemented; session middleware pending |
| [013](013-access-token-storage.md) | Opaque access-token digest storage per session | Storage and issuance implemented; bearer authentication pending |
| [014](014-initial-session-issuance.md) | Issue opaque token families from current credential snapshots | Lookup, issuance and HTTP login implemented |
| [015](015-secure-login-http.md) | Public login route and shared bounded credential admission | Implemented; bearer authentication pending |
| [019](019-typed-api-errors.md) | Typed safe errors and server-generated diagnostic request IDs | Accepted; own/independent verification and native acknowledged review passed; [PR #59](https://github.com/cuenora/cuenora/pull/59) integrated into `main` (`3bb2d07`) |
| [021](021-session-logout.md) | Refresh-possession logout and reauthenticated global session revocation | Implemented in `392cb27`; own/independent verification passed; native review acknowledged; [PR #65](https://github.com/cuenora/cuenora/pull/65) merged into `main` (`588d0c4`), with full published-tree equality verified |

Add a new numbered ADR when changing an accepted decision. Link the superseded
record rather than erasing its rationale. Roadmap tasks remain in the
[master plan](../../plans/master-plan.md).
