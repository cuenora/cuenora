# ADR-012: Bounded password verification and opaque login credentials

- **Status:** Accepted; credential primitives implemented, login/session issuance pending
- **Date:** 2026-10-26
- **Scope:** P04.03 credential slice; extends ADR-009 and ADR-005

## Decision

Reuse the existing stored Argon2id credentials and email policy. Verify exact
password bytes without trimming, Unicode normalization or reapplying registration
minimum-length/common-password policy. Login input is bounded to 128 Unicode
scalar values (at most 512 UTF-8 bytes). This permits verification of an existing
credential even if later signup rules become stricter.

Before expensive work, validate the bounded P04.02 PHC profile: Argon2id v19,
19,456 KiB, two iterations, one lane, 32-byte output and at least 16 salt bytes.
Reject malformed, excessive or unsupported stored parameters with a fieldless
unavailable error, never an embedded hash or parser error. Future hash-profile
changes need an explicit compatibility/update policy; blindly trusting stored
PHC work factors would defeat CPU/memory admission limits.

For a missing identity, verify a public synthetic dummy hash with the same work
profile and always return false, including when its known dummy password matches.
This mitigates a major missing-user timing shortcut; it does not promise identical
network timings. Caller code must share registration's bounded admission and run
verification on blocking threads. The pure helper does neither HTTP nor scheduling.

The maintainer approved opaque access credentials lasting 900 seconds and opaque
refresh credentials lasting 2,592,000 seconds. Generate 64 bytes with the OS CSPRNG
and divide them into independent 256-bit secrets. Emit lowercase hexadecimal
with `ca1_` and `cr1_` purpose/version prefixes; clients must treat the complete
values as opaque. Persist SHA-256 digests of the complete emitted strings, not
raw credentials. A fallible entropy read must produce either the complete pair
or no credentials. No clock or persistence lives in these primitives.

Raw credential types deliberately provide no Debug, Display or Serialize
implementation. An explicitly named exposure method is reserved for the eventual
successful authentication response. This reduces accidental logging; it is not
a memory-zeroization guarantee. SHA-256 uses RustCrypto's already-locked sha2
0.10 package rather than a custom hash implementation; only a direct dependency
edge is added, without adding/upgrading package versions.

## Tradeoffs and scope

Opaque credentials avoid JWT signing-key configuration but require server-side
state and later validation lookup. Initial session issuance will be atomic across
session/access/refresh rows. A new forward access-token migration and public login
are still pending in later slices; no endpoint or session exists merely because
these helpers exist. Refresh rotation/revocation remains P04.04, identity
middleware P04.05, and email verification/recovery P04.12. Pending users may log in
without financial authority; these primitives do not authorize financial access.

## Verification

Focused tests exercise existing signup hashes, exact Unicode/space/NUL bytes,
legacy-password verification independent of signup screening, missing-user dummy
matching, and bounded rejection of malformed/excessive PHC values. Credential
tests exercise all 64 entropy bytes, purpose-separated output, distinct OS-random
pairs, exact SHA-256 digests, approved durations and partial entropy failure.
Compile-fail documentation tests prevent accidental credential formatting.
Observed results are recorded in the P04.03 execution plan and ODD logbook.

Sources: [OWASP session management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html),
[RustCrypto PHC parsing](https://docs.rs/password-hash/0.5.0/password_hash/struct.PasswordHash.html),
and [RustCrypto SHA-2](https://docs.rs/sha2/0.10.9/sha2/).
