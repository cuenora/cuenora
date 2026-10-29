# P05.02 — Account creation
Implement authenticated, verified-email account creation with lifetime idempotency; uniformly version the API.
Branch: `feat/p05-02-account-creation`. Delivery: exception-ok, single PR explicitly approved (about 1,500 authored lines). Runner: pinned Rust 1.99.1 and isolated PostgreSQL harness.

## Specs

- S1: “Comencemos con la tarea P05.02 del plan maestro” — “Implement account creation with the approved account/name model and currency, deriving ownership from authenticated identity and enforcing verified email; cover rejected attempts without writes.” Opening balances remain P05.03; verification delivery/challenge runtime remains explicitly pending, never bypassed.
- S2: “No, estamos desarroyando el producto, no tenemos usuarios reales aun, este es el momento de hacer cambios, si vamos a vercionar, vercionamos todos los endpoints” — move all implemented auth and health routes under `/v1`, without legacy aliases. Keep their other behavior unchanged.
- S3: User approved “exigir `Idempotency-Key`: misma clave y contenido devuelve el resultado original; misma clave con contenido diferente devuelve conflicto.” Verify replay, conflict, separate owners/keys and simultaneous retries through HTTP against isolated PostgreSQL.
- S4: User approved “conservar la clave durante toda la vida de la cuenta: evita que un reintento tardío cree otra cuenta, a cambio de guardar un registro adicional por creación.” Persist atomically with creation, no TTL expiry.

## Tasks

- T1 [S2] inline (one understood route migration; no writer trigger), implemented and independently verified; commit preparation authorized under accepted size exception. Risk: item 3, existing API path contract.
- T2 [S1,S3,S4] inline (single dependent behavior, no independent parallel writer units), implemented and independently verified; commit preparation authorized under accepted size exception. Risk: items 2 and 4, verified ownership and concurrent retries; item 5, additive migration and domain dependency.
- T3 [S1–S4] delegated verification after own checks (named high-risk items above), PASS S1–S4; native review pending; single-PR size exception approved. Review/chain choices remain human-owned.

## Log

- L1: “Comencemos con la tarea P05.02 del plan maestro”
- L2: “No, estamos desarroyando el producto, no tenemos usuarios reales aun, este es el momento de hacer cambios, si vamos a vercionar, vercionamos todos los endpoints”
- L3: “Sí” — approved mandatory idempotency key, same-content replay and changed-content conflict.
- L4: “Sí” — approved retention for the lifetime of the account.
- L5: Baseline `8b1834d`; only pre-existing untracked `.atl/` preserved. Read-only mapping and parent spot checks found exact-path throttling, no account adapter, no verification runtime and no idempotency table. All source writes follow this plan. No remote operations authorized.
- L6: Proposed technical contract: strict JSON `name`, `account_type`, `currency`; normalized domain name is canonical content; opaque case-sensitive 1–128 ASCII alphanumeric/`-`/`_` key scoped to authenticated owner and this operation. Response 201 with original account metadata, 409 conflict, 403 unverified; standard safe errors/no-store. Reject unknown authority/balance fields. These are implementation bounds, not a new account product policy.

- L7: RED observed: new versioning test failed on legacy health 200 vs expected 404; account bearer test failed because route was absent; six isolated DB tests failed before account route/table implementation. GREEN: versioning and bearer tests passed after implementation. The first DB suite had 47/48 infrastructure cases pass, with five/six new account cases passing; the last failed because the test itself seeded an extra user after taking its user-table snapshot. Moving fixture preparation before the snapshot fixed it without production changes.
- L8: Own checks: repository validation, pinned Rustfmt and Clippy passed. Full Cargo suite passed (75 ordinary/doctests, 45 ignored before adding independent probe). The final Cargo rerun passed (75 passed, 46 ignored); the final focused PostgreSQL target passed all seven account cases. The full 48-case infrastructure suite was not rerun after the fixture-only correction; the other 47 cases had passed. No failures suppressed.
- L9: Independent verifier PASS S1–S4 in fresh baseline/candidate scratch copies. Seven PostgreSQL cases including added key-header/unverified-replay probes passed; eight HTTP differential cases matched baseline normalized responses under the explicitly authorized prefix mapping. Parent retained the new regression probe, reran the focused seven-case harness and Cargo/Clippy, and fixed an advisory stale OpenAPI sentence. Successful DB-backed auth flows were covered by the original full harness, not by the deterministic differential probe. Parent spot check: versioning test passed.
- L10: No safe slice pass can make the complete account-creation behavior (transactional persistence, HTTP, security/concurrency tests and contract) fit 400 lines without artificial separation. Uniform versioning is separable, but the creation slice still needs a size exception. Total current authored diff about 1,500 lines including new tests/docs. Delivery strategy decision required before any commit: chain the versioning and account creation units (with exception for larger creation unit), or explicit single-PR size exception. No commit, push, PR or native review transaction started. RDD status is on (global).
- L11: Remaining email verification/challenge/delivery runtime is deliberately not implemented; no bypass or end-to-end readiness claim. P05.02 master checkbox remains open until delivery/review decisions are resolved. Opening movements remain P05.03.

- L12: “Sí” — user explicitly authorized the proposed single-PR size exception after disclosure of 1,517 authored lines. Use one coherent versioned-account-creation work unit for T1/T2, with code/tests/contracts together; no chain. No remote push or PR creation is authorized by this answer. Local ODD work-unit commit and native review continue. Rollback boundary: this new creation endpoint/migration and uniform route-prefix migration with their tests/contracts, leaving pre-existing financial/auth policies unchanged.
