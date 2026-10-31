# Architecture decisions

ADRs record chosen direction and its tradeoffs; acceptance does not mean every
component has been implemented. The index summarizes current implementation as of
2026-10-29; original context and scope in individual records remain historical.
Use the [master-plan snapshot](../../plans/master-plan.md#current-delivery-snapshot)
for delivery progress, not an old ADR's deferred-work list.

| ADR | Decision | Status |
| --- | --- | --- |
| [001](001-initial-stack.md) | Initial backend, mobile and delivery stack | Accepted |
| [002](002-exact-money.md) | Exact recorded money, bounds and quantization | Accepted; domain types implemented |
| [003](003-date-semantics.md) | Business dates, UTC audit instants and stable named zones | Accepted design; runtime implementation pending |
| [004](004-first-party-identity.md) | First-party credentials, open registration and email verification | Accepted; authentication implemented, verification/recovery runtime pending |
| [005](005-identity-storage.md) | Case-insensitive ASCII/IDNA identity and opaque refresh-session storage | Schema and authentication/session runtime implemented |
| [006](006-append-only-account-movements.md) | Append-only account postings and compensating corrections | Accepted; storage implemented, correction workflow pending |
| [007](007-minimal-transaction-audit.md) | Field-name-only append-only transaction audit | Accepted; storage implemented, automatic auditing pending |
| [008](008-account-reference-compatibility.md) | Account type/currency changes preserve existing financial references | Accepted; native storage constraints |
| [009](009-registration-credentials.md) | Registration input policy and salted Argon2id credentials | Registration primitives integrated; login also implemented |
| [010](010-atomic-registration-storage.md) | Atomic pending registration without duplicate credential replacement | Implemented; called by registration HTTP |
| [011](011-bounded-registration-http.md) | Bounded local registration HTTP and explicit database configuration | Implemented and independently verified |
| [012](012-login-credential-primitives.md) | Bounded verification and initial opaque login credentials | Primitives and HTTP login implemented; middleware covered by ADR-018 |
| [013](013-access-token-storage.md) | Opaque access-token digest storage per session | Storage/issuance implemented; bearer identity covered by ADR-018 |
| [014](014-initial-session-issuance.md) | Issue opaque token families from current credential snapshots | Lookup, issuance and HTTP login implemented |
| [015](015-secure-login-http.md) | Public login route and shared bounded credential admission | Implemented; route-scoped identity covered by ADR-018 |
| [016](016-sliding-refresh-rotation.md) | Sliding refresh rotation, consumed-token reuse and family revocation | Implemented; no absolute family-age cap |
| [017](017-refresh-http.md) | Bounded public refresh HTTP | Implemented |
| [018](018-authenticated-identity.md) | Route-scoped opaque bearer identity | Implemented; does not implement financial ownership or verification gates |
| [019](019-typed-api-errors.md) | Typed safe errors and server-generated diagnostic request IDs | Accepted; own/independent verification and native acknowledged review passed; [PR #59](https://github.com/cuenora/cuenora/pull/59) integrated into `main` (`3bb2d07`) |
| [020](020-authentication-throttling.md) | Bounded process-local authentication throttling and explicit proxy trust | Implemented; single-instance scope |
| [021](021-session-logout.md) | Refresh-possession logout and reauthenticated global session revocation | Implemented in `392cb27`; own/independent verification passed; native review acknowledged; [PR #65](https://github.com/cuenora/cuenora/pull/65) merged into `main` (`588d0c4`), with full published-tree equality verified |
| [022](022-email-verification-and-password-recovery.md) | Session-bound email verification and password recovery | Accepted design; runtime, migration and email delivery not implemented |

| [023](023-versioned-account-creation.md) | Uniform `/v1` and lifetime account-creation idempotency | Accepted; implemented locally, own/independent checks and native review passed; not yet published |

| [024](024-dated-opening-balances.md) | Explicit profile zones and atomic dated opening balances | Implemented locally; own/independent checks passed; native review pending |

| [025](025-owner-scoped-account-listing.md) | Verified-owner account metadata and bounded cursor reads | Implemented locally; own/independent checks passed; native review pending |

| [026](026-exact-account-balances.md) | Exact current balances in owner-scoped account pages | Own/independent checks passed; native review acknowledged |

| [027](027-atomic-account-management.md) | Atomic owner-scoped rename, archive and reactivation | Own/independent proof passed; native review acknowledged |

| [028](028-authenticated-category-catalog.md) | Authenticated shared category reads without verification | Own/independent checks passed; native review acknowledged |

| [029](029-accountless-income-retries.md) | Permanent accountless-income retry equivalence | Implemented locally; native review skipped for this candidate by maintainer choice; publication pending |

| [030](030-accountless-expense-retries.md) | Independent permanent accountless-expense retry keys | Own/independent checks passed; native review skipped for this candidate; publication pending |

| [031](031-owner-scoped-transaction-history.md) | Bounded owner-scoped recorded history with composable filters | P05.12 implemented in `1b1b526`; own/independent checks passed; native review skipped for this candidate |

| [032](032-atomic-transaction-corrections.md) | Atomic corrections, linked inverse postings and permanent mutation retries | P05.13 own/independent checks passed; native review skipped for this candidate by maintainer choice |

| [033](033-exact-transfer-quotes.md) | Canonical entered quotes, checked comparisons and no-quote retry compatibility | Implemented locally; own/independent checks passed; publication pending |

Add a new numbered ADR when changing an accepted decision. Link the superseded
record rather than erasing its rationale. Roadmap tasks remain in the
[master plan](../../plans/master-plan.md).
