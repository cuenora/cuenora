# ADR-009: Registration credential policy

- **Status:** Accepted; credential primitives only, registration endpoint pending
- **Date:** 2026-10-25
- **Scope:** P04.02; hashing prerequisite from P04.03, not login

## Decision

Keep transport-independent credential policy in the API library's
`registration_policy` module; the monetary domain remains dependency-free.
Use the existing user/credential schema. Original email input is preserved;
ASCII mailbox dot-atoms are supported, with strict IDNA domain conversion and
lowercase transport domains. Display names, quoted local parts, IP literals,
single-label domains and trailing dots are outside this initial input surface.
Never trim input, remove dots/plus tags, or lowercase the stored local part.
PostgreSQL's generated lowercase key remains the identity-comparison authority.

The user approved 15–128 Unicode scalar values per password, including spaces
and Unicode, without composition requirements. Hash exact UTF-8 bytes: no trim,
normalization, truncation or control-character removal. The offline common-password
screen checks a lowercased copy against the embedded list in `passwords` 3.1.19,
so mixed-case variants cannot bypass it. This list is finite, not a claim to detect
every compromised password. No entropy threshold or remote password lookup is used.

Use RustCrypto `argon2` 0.5.3, Argon2id v19, 19 MiB memory, two iterations,
one lane and a 32-byte output. Generate a fresh 16-byte salt with the OS CSPRNG
for every hash; entropy/hash failures return only a safe category. Encoded PHC
strings carry parameters for later verification/upgrades. `idna` 1.1.0 provides
strict IDNA processing. Versions are recorded in Cargo.lock.

## Tradeoffs and boundary

The embedded MIT-licensed password list avoids shipping a separate large corpus
or sending credentials to another service, but adds a dependency/table and needs
review when updated. IDNA and Argon2 implementations are reused, not handwritten.
RustCrypto is MIT/Apache-2.0; IDNA is MIT/Apache-2.0. No HTTP or persistence code
depends on the credential primitives in this slice yet.

The hashing parameters match an OWASP minimum, not a production capacity claim.
The endpoint must bound body size/admission, run hashing off async executors and
retain admission until work actually ends, including cancellation. Benchmarking
and deployment-level abuse controls remain necessary before public exposure.
Verification mail/challenges, login and sessions remain outside P04.02.

## Sources and checks

- [OWASP password storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [RustCrypto Argon2 0.5.3](https://docs.rs/argon2/0.5.3/argon2/)
- [IDNA strict conversion](https://docs.rs/idna/1.1.0/idna/fn.domain_to_ascii_strict.html)
- [Password list API/source and MIT license](https://github.com/MagicLen/passwords)

Run `rustup run 1.99.1 cargo test -p cuenora-api --test registration_policy --locked`
from `backend/`. Tests cover email preservation/rejection, Unicode length bounds,
spaces/controls, common variants, distinct salts, exact-byte verification and
wrong-password rejection. Whole-feature verification is tracked in the
[execution record](../../plans/active/2026-10-25-registration.md).
