# Shared JSON contract fixtures

`listing-responses.json` is the canonical synthetic corpus for P10.11. Each
top-level key contains one complete API response, not a new API envelope:

- `accounts`: active/archived metadata, signed i64-bound balances and a cursor.
- `accountless`: posted USD income above binary64's exact integer-minor-unit
  range, leap-day date, explicit null relationships and a terminal page.
- `transfer`: actual SEK/EUR amounts plus flattened informational quote metadata.

Rust unit tests serialize the existing listing response types and compare whole
JSON values. Transaction tests also use the production row conversion, money
formatting and quote calculation. KMP common tests pass these same examples
through `ApiClient` and the public HTTP repositories using MockEngine.

Gradle embeds the corpus's UTF-8 bytes in generated commonTest Kotlin under
`mobile/shared/build/generated/contractFixtures/`. It declares the JSON input
and generated output so edits invalidate tests on both targets. No resource
loader, committed Kotlin copy or production dependency is needed.

From `backend/`: `cargo test -p cuenora-api --lib shared_json --locked`.
From `mobile/`, with the [documented toolchain](../../mobile/README.md):
`./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test`.

These are serializer/parser contract checks, not PostgreSQL, live HTTP, OpenAPI
schema validation, UI or all-endpoint coverage. Existing route integration tests
remain separate. When changing a covered contract, update the corpus and both
sides' independently specified expectations together; do not generate expected
models from the fixture being asserted.
