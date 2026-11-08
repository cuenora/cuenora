# Mobile foundation and native hosts

P10.01 introduces `:shared`: Kotlin common source compiled to an Android AAR and
static `CuenoraShared` frameworks for iOS device ARM64 and simulator ARM64.
P10.02 adds minimal [Android Compose](androidApp/README.md) and
[iOS SwiftUI](iosApp/README.md) hosts displaying the shared marker. These are
integration shells, not financial screens. P11.01 extends the Android host with
a native navigation/theme/component foundation, still without financial screens
or financial resource wiring. P11.02 adds native Android access and a shared
authentication/session lifecycle. P11.03 adds first-account onboarding and P11.04
adds paginated account lists and read-only metadata/balance details; iOS remains a minimal host. P10.03 adds shared
`commonTest` tests.
Intel iOS simulators are not targeted.

## Configure and build

Open **this `mobile/` directory** in Android Studio. For terminal builds on macOS,
select the existing Android Studio JBR and your installed SDK for that shell:

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_HOME="$HOME/Library/Android/sdk"
cd mobile  # from the repository root
./gradlew --version
./gradlew :shared:assembleAndroidMain \
  :shared:linkDebugFrameworkIosArm64 \
  :shared:linkDebugFrameworkIosSimulatorArm64
```

Use your own installation paths when different. Do not commit `local.properties`
or machine-specific paths. No global Gradle installation is required. The first
build downloads the checksum-pinned Gradle distribution, plugins and Kotlin/Native
toolchain. Build outputs and caches are ignored by Git. iOS compilation requires
macOS with full Xcode selected, but does not require signing or a running simulator.
Android AAR compilation does not require an emulator.

Build the Android host from `mobile/`:

```bash
./gradlew --no-daemon :androidApp:assembleDebug :androidApp:lintDebug
```

Open `iosApp/Cuenora.xcodeproj` for the iOS host; its [instructions](iosApp/README.md)
include unsigned command-line builds and the Java environment required by its
pre-Sources Gradle phase. For automated Android checks, use an external temporary
`ANDROID_USER_HOME` to isolate the generated debug key. No release-signing
configuration is checked in.

## Shared unit tests

From `mobile/`, with the Java and Android SDK environment above:

```bash
./gradlew --no-daemon :shared:testAndroidHostTest :shared:iosSimulatorArm64Test
```

Both tasks run `SharedModuleTest` from `shared/src/commonTest/kotlin/`, asserting
the existing `SharedModule.name` marker. Android uses local JVM host tests,
explicitly enabled with `withHostTest {}`; no Android emulator is needed.
iOS uses the Kotlin/Native ARM64 simulator test runner and requires macOS/Xcode
with an installed iOS simulator runtime. These are shared-library unit tests,
not device, UI, backend or end-to-end tests.

Run the shared verification lifecycle with `./gradlew --no-daemon :shared:allTests`.
Check the per-target XML reports under `shared/build/test-results/`: a successful
build or a skipped device-target task is not proof that tests actually ran.
`iosArm64` device execution is not part of this local simulator-based check.
CI runs these targets separately; see [shared mobile CI](#shared-mobile-ci-p1012).

P10.11 adds [shared JSON contract fixtures](../contracts/fixtures/README.md)
for account pages, accountless income and quoted cross-currency transfers.
Rust checks actual response serializers; commonTest feeds the same corpus through
the HTTP repositories on Android-host and iOS-simulator. Gradle embeds the JSON
only in generated test sources, never in the production AAR/frameworks.

On 2026-11-07 the marker test passed once per target, with zero failures, errors
or skipped cases, both locally and in an independent clean snapshot. All three
shared artifact builds also passed; Android marker bytecode and both iOS exported
headers matched the baseline. Native linking emitted non-fatal missing debug-cache
object warnings; the Xcode compatibility limitation below still applies.

If a build restored from another toolchain location fails with duplicate native
KLIB names, run `./gradlew --no-daemon :shared:clean` and repeat the tests before
considering configuration changes. Do not suppress Kotlin/Native diagnostics.

See the [Android KMP test configuration](https://developer.android.com/kotlin/multiplatform/plugin)
and [Kotlin shared-test guide](https://kotlinlang.org/docs/multiplatform/multiplatform-run-tests.html).

## Shared mobile CI (P10.12)

The existing [CI workflow](../.github/workflows/ci.yml) adds two shared-library
checks on every pull request, push to `main`, scheduled run and manual dispatch:

| Check | Runner | Task |
| --- | --- | --- |
| Shared / Android host | `ubuntu-24.04` | `:shared:testAndroidHostTest` |
| Shared / iOS simulator | `macos-26` (ARM64) | `:shared:iosSimulatorArm64Test` |

Both use the runner-provided Java 25 and the checksum-pinned Gradle wrapper.
The pinned Ubuntu/macOS images expose `JAVA_HOME_25_X64` and
`JAVA_HOME_25_arm64`, respectively. CI validates the executable and exact major
before publishing `JAVA_HOME`/`PATH`; a missing or wrong-major runtime fails
closed instead of fetching a different runtime. This avoids the redundant JDK
release download that failed with HTTP 504 even while the runner listed Java 25.
See the official [Ubuntu inventory](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)
and [macOS ARM64 inventory](https://github.com/actions/runner-images/blob/main/images/macos/macos-26-arm64-Readme.md).

Gradle 9.5 stays pinned to its existing SHA-256 and HTTPS distribution URL.
Its native wrapper downloads now use a 60-second network timeout, three retries
(four attempts total), and 1-second initial exponential backoff. Only bootstrap
is retried, not failed test execution. Persistent download failure and checksum
mismatch still exit nonzero, and the existing JUnit/artifact guards stay strict.
See [Gradle 9.5 retries](https://docs.gradle.org/9.5.0/release-notes.html).

SDK packages
`platforms;android-36`, `platforms;android-37.0` and build tools 36/37 are ensured
because the build configures both Android modules. The 37.0 suffix is part of
Google's package ID; `platforms;android-37` is not published. iOS selects Xcode
26.4.1 through `DEVELOPER_DIR`, not a global switch.
This matches Kotlin's [26.4 compatibility baseline](https://kotlinlang.org/docs/multiplatform/multiplatform-compatibility-guide.html)
and the [macOS runner inventory](https://github.com/actions/runner-images/blob/main/images/macos/macos-26-arm64-Readme.md).
If the image drops that Xcode, the job fails rather than silently using another.

The matrix does not cancel the other platform when one fails. Checkout still
disables persisted credentials; token permissions remain read-only. Gradle's basic GitHub cache is
read-only off `main`; no build scan, signing or publishing is enabled.
JUnit XML and HTML reports are retained for seven days even after test failure.
Report validation requires nonzero executed cases and no failures/errors/skips;
missing, malformed or inconsistent XML fails the job. No fixed test count is
encoded, so new shared tests are included automatically.

Reproduce the report guard from the repository root after running target tests:

```bash
python3 scripts/check_mobile_test_results.py mobile/shared/build/test-results/testAndroidHostTest
python3 scripts/check_mobile_test_results.py mobile/shared/build/test-results/iosSimulatorArm64Test
python3 tests/infrastructure/test_mobile_test_results.py -v
python3 tests/infrastructure/test_mobile_bootstrap.py -v
```

The bootstrap regression suite requires `java` and `javac` (or a configured
`JAVA_HOME`), uses the real committed wrapper JAR with an isolated loopback HTTP
server, and checks recovery after 504, bounded exhaustion, initial success and
checksum rejection. A tiny synthetic launcher is only a download fixture, never
evidence that Gradle builds or financial tests executed. Separate workflow-shell
probes check both runner architectures, missing Java and wrong-major rejection.

These jobs do not run Android instrumentation, app-hosted Keychain, UI, device,
backend or E2E tests. Local checks do not prove GitHub runner execution; record
both hosted job results before marking P10.12 complete.

## Shared authentication and native Android access (P11.02)

`org.cuenora.shared.auth.AuthRepository` exposes typed register/login/refresh/logout
operations over ApiClient without owning its lifecycle. It needs no OwnerId and
never decodes opaque tokens. Registration expects generic `202 accepted`, login
and refresh require a complete Bearer pair plus integer positive lifetimes, and
logout expects `204`. Failures stay typed and no request is automatically retried.

`AuthSession` is one process-owned coordinator per backend/store. It reuses shared
form validation, publishes credential-free state and stores through SessionStore.
The entire refresh/storage transaction is serialized; concurrent due checks do
not reuse a refresh generation. Saved pairs are removed before rotation to avoid
crash/lost-response replay, trading seamless offline restoration for fresh sign-in
after interruption. Logout distinguishes local removal from remote confirmation.
Android owns the process instance; a future iOS UI must own its equivalent rather
than instantiate competing coordinators. Financial repositories remain unconnected.

See [Android usage](androidApp/README.md#authentication-and-session-lifecycle),
[ADR-057](../docs/architecture/decisions/057-native-authentication-lifecycle.md) and
[execution evidence](../docs/plans/active/2026-11-08-android-authentication.md).
Shared tests use MockEngine/fakes and both existing target runners. No live
backend authentication, physical-device or financial workflow proof is implied.

## Secure session storage

`org.cuenora.shared.session.SessionStore` defines suspend `read`, `replace` and
`clear` for a complete access/refresh pair. Absence is null; failures throw a
redacted `SessionStoreException` (Corrupt or Unavailable). Cancellation propagates.
Do not log raw credential fields; the wrapper's display is redacted, not the strings.

Android uses `AndroidSessionStore(context)` with AndroidKeyStore AES-256-GCM and
an atomic encrypted record under the application's no-backup directory. iOS uses
`KeychainSessionStore()` with one device-only, non-synchronizable Keychain item.
Both default their namespace to `BackendConfig.BASE_URL`; pass a matching custom
namespace when using another backend. Read/replace/clear operations serialize
within this process off the caller thread. This does not serialize HTTP refresh,
perform logout, promise zeroization or require biometric authentication.
See [ADR-053](../docs/architecture/decisions/053-native-session-storage.md).

On 2026-11-07, 32 Android-host and 33 iOS-simulator tests passed, as did allTests,
five Android API37 instrumentation tests and six app-hosted Keychain XCTest cases.
Android AAR/test APK and both iOS frameworks built. The original CLI Keychain
runner reported unavailable storage; the real service cases now run in the existing
iOS app host. App-host tests also exposed a CFNumber/CFBoolean bridge bug, now
corrected and pinned by a deterministic Native test. No security fallback was added.

With an explicitly selected isolated Android emulator, from `mobile/`:

```bash
./gradlew --no-daemon :shared:assembleAndroidDeviceTest
ANDROID_SERIAL=emulator-5580 ./gradlew --no-daemon :shared:connectedAndroidDeviceTest \
  -Pandroid.testInstrumentationRunnerArguments.class=org.cuenora.shared.session.AndroidSessionStoreTest
```

Use the serial of your isolated test emulator, not a personal device. JVM tests
do not exercise Keystore. Run the separate [app-hosted iOS tests](iosApp/README.md#app-hosted-keychain-tests)
to verify Keychain: Gradle allTests checks common logic and the CFBoolean boundary,
not app-hosted service behavior. P11.02 subsequently adds shared login/refresh orchestration and Android host wiring;
the P10.07 proof above remains storage-only. No physical-device, backup/restore or process-crash proof is claimed.

## Shared HTTP transport

P10.06 adds `org.cuenora.shared.network.ApiClient` and typed `ApiResult`/`ApiFailure`.
Ktor 3.6.1 uses Android/Darwin engines; tests inject MockEngine. See
[ADR-052](../docs/architecture/decisions/052-shared-http-client.md) for failure and lifecycle rules.

```kotlin
val api = ApiClient() // Uses BackendConfig.BASE_URL; construction makes no request.
try {
    val result = api.request("/v1/categories") // Raw transport; resource repositories provide typed operations.
    // Handle Success or Failure; do not log response bodies or credentials.
} finally {
    api.close()
}
```

Use a custom HTTPS origin, or `http://127.0.0.1:3000` for local development.
Paths start with `/v1/` and may include an encoded query. Requests default to GET;
pass `method`, an optional JSON `body`, and caller-owned `headers` as needed.
Use the exact amount serializers below when building JSON financial fields.
No authentication/session orchestration, automatic redirects, application retries,
cookies or payload logging are installed. The request timeout is 30 seconds.
Cancellation propagates instead of becoming a recoverable failure.

HTTP failures retain status, optional parsed code/message, request ID and valid
integer Retry-After seconds. Unknown codes survive; HTML/malformed errors remain
HTTP failures without a parsed envelope. Invalid successful JSON, network I/O and
timeouts have separate failure types. Success body is null for 204; JSON `null`
otherwise remains `JsonNull`. Close every client; also close injected engines,
which remain caller-owned. Mock tests do not contact the configured backend.
Resource operations use the repositories below. Session orchestration and host
integration remain subsequent work.

On 2026-11-07 all 28 tests per Android-host/iOS-simulator target, `allTests`,
all three shared artifacts and independent baseline/candidate verification passed.
The timeout regression takes about 30 seconds per target. Existing Xcode and
native debug-cache warnings remain; no remote backend or device network was tested.

## Account and transaction repositories

P10.08 adds `org.cuenora.shared.repository` interfaces and HTTP adapters for
listing, account creation/update (including archive/reactivate), and transaction
income/expense/interest/transfer creation, editing and voiding.
See [account usage](shared/docs/account-repository.md),
[transaction usage](shared/docs/transaction-repository.md) and
[ADR-054](../docs/architecture/decisions/054-shared-resource-repositories.md).

Supply an explicit `RepositoryContext(ownerId, accessToken)` and caller-owned
`ApiClient`. Context identity completes domain mapping; only the server grants
authority. Replace repository instances when the session/user changes. The caller
closes the client, retains mutation keys for explicit retries and handles typed
`RepositoryResult` failures. No automatic refresh, retry, local persistence or host
network wiring is added. Account balance is separate from account metadata;
transaction edit/void return acknowledgements, not snapshots.

Common tests include configurable `FakeAccountRepository` and
`FakeTransactionRepository`. These capture calls and return scripted typed results;
they do not simulate server financial or authorization rules. Both targets execute
the same MockEngine/public-interface contract tests with synthetic data only.

## Shared financial domain

P10.04 adds `Money`, `Account`, `Transaction` and `SavingsForecast` under
`org.cuenora.shared.domain`; see [ADR-050](../docs/architecture/decisions/050-shared-financial-domain-models.md)
for their invariants and boundaries. Recorded money uses signed `Long` hundredths
with explicit currency. Forecast rationals/projected minor units retain arbitrary
precision separately, without arithmetic or conversion to recorded balances.
Models use typed opaque identities and Gregorian business dates, not HTTP/JSON
fields, system clocks or platform APIs. Constructor checks do not authorize
operations or replace server rules. Resource client mapping, form validation
and financial use cases remain separate tasks.

On 2026-11-07 all nine shared tests per target passed (zero failures/errors/skips),
along with `allTests` and all three shared artifact builds. This includes the
existing marker regression test. Native debug-cache warnings remain non-fatal.

## Exact JSON amount adapters

P10.05 adds serializers in `org.cuenora.shared.serialization`, without changing
or annotating the domain models. See [ADR-051](../docs/architecture/decisions/051-exact-json-amount-adapters.md).
Use explicit serializers for amount **fields**, not entire API resources:

```kotlin
val serializer = MoneyAmountSerializer(Currency.SEK)
val wire = Json.encodeToString(serializer, Money(12345, Currency.SEK)) // "123.45" (JSON string)
val money = Json.decodeFromString(serializer, wire)
```

`MoneyAmountSerializer(currency)` accepts the recorded input grammar and writes
canonical two-decimal strings. Currency comes from the resource; mismatches,
overflow and excess precision fail without rounding. JSON numbers are rejected.
`ProjectedAmountSerializer` maps arbitrary-precision projected minor units to
nonnegative two-decimal strings; use `.nullable` for an open tail.
`ReportAmountSerializer` validates signed two-decimal report strings without
narrowing or changing their text. None uses floating point or locale formatting.
Rates and rational accrual integer strings are not monetary amount fields.

On 2026-11-07 all 14 tests per Android-host/iOS-simulator target, `allTests`,
all three shared artifacts and independent snapshot verification passed. Existing
Xcode/debug-cache limitations still apply.

The JSON runtime is pinned at 1.11.0; handwritten serializers require no compiler
plugin. HTTP transport is supplied by P10.06; resource repositories/DTO mapping
and cross-language fixtures remain P10.08/P10.11. Never log financial payloads
or raw JSON parser exceptions.

## Backend base URL

The single default origin is `BackendConfig.BASE_URL` in
[`BackendConfig.kt`](shared/src/commonMain/kotlin/org/cuenora/shared/config/BackendConfig.kt).
To switch the shared default from development to production, edit that value and
rebuild both Android and iOS apps. It is build-time configuration, not a remote
setting; already-installed apps retain their compiled URL. Do not duplicate the
default in platform hosts. `ApiClient(baseUrl = "https://api.example.com")` still
allows explicit overrides for tests or local development.

The transport uses this origin and the versioned API under `/v1`.
Minimal P10.02 hosts still send no requests. A configured origin does not verify
server availability or grant remote administration access.

## Pinned baseline

| Component | Selection |
| --- | --- |
| Kotlin Multiplatform | 2.4.21 |
| Android KMP library plugin | 9.3.1 |
| Gradle wrapper | 9.5.0 |
| Gradle runtime | Java 25 (local Android Studio JBR 25.0.3) |
| Android compile / minimum SDK | 36 / 23 |
| Android host compile / target / minimum SDK | 37 / 36 / 23 |
| Android JVM bytecode | 17; separate from the JVM running Gradle |
| Ktor client / Android and Darwin engines | 3.6.1 |
| Shared test coroutines | 1.11.0 |

Minimum Android SDK 23 and iOS 15 are provisional development baselines, not a
promised app support policy. Compose is confined to the Android host; the shared
module adds kotlinx.serialization JSON 1.11.0 for amount adapters and Ktor for
transport. Domain sources remain independent of both.
See [ADR-048](../docs/architecture/decisions/048-shared-mobile-build.md) and
[ADR-049](../docs/architecture/decisions/049-native-host-integration.md).

Kotlin's [published compatibility matrix](https://kotlinlang.org/docs/multiplatform/multiplatform-compatibility-guide.html)
lists Xcode 26.4 for this Kotlin version. Installed Xcode 27.2 is outside that
matrix: local compilation results do not establish official support. Do not
suppress compatibility checks. If an older Xcode is needed, install it separately
and select it per process using `DEVELOPER_DIR`, not a global selection change.

## Artifacts and next steps

Android output is under `shared/build/outputs/aar/`; Apple outputs are under
`shared/build/bin/iosArm64/debugFramework/` and
`shared/build/bin/iosSimulatorArm64/debugFramework/`.
Both retain the `SharedModule.name` marker and expose the P10.04 domain models;
these models do not implement financial workflows.
On 2026-11-06, all three builds passed locally using JBR 25.0.3 and Xcode 27.2.
Inspection confirmed the marker in Android bytecode and both framework headers;
Apple archives identify iOS device/simulator separately with minimum iOS 15.0.
The [execution plan](../docs/plans/active/2026-11-06-kmp-foundation.md) records
observed commands, limitations and verification. This does not complete Phase 10
or prove runtime UI behavior. The [P10.02 execution record](../docs/plans/active/2026-11-06-native-hosts.md)
tracks host verification separately. CI and E2E setup remain P10.12 and Phase 13.

## Shared form validation

P10.09 adds opt-in, UI-independent functions in
`org.cuenora.shared.validation` for account, transaction, authentication,
recurrence and savings configuration forms. Results contain only field names
and stable issue codes, not submitted values or localized UI messages.
`isLocallyValid` means the implemented local checks passed;
`serverChecksRequired` is always true, including for locally valid forms.

Passwords are never trimmed or normalized. Registration and password reset
check the 15–128 Unicode-scalar length policy; login deliberately does not apply
the signup minimum or common-password policy. The server checks common passwords.
Email checks are advisory: IDNA conversion and canonical address constraints
remain server-side. Do not log form inputs, passwords, tokens or email addresses.

Financial amounts use exact dot-decimal input, never binary floating point,
localized grouping or rounding. Account names reuse domain normalization;
business dates are strict Gregorian dates. Validate complete merged edit forms:
the caller still distinguishes preserve, clear and replace when building a
repository PATCH. Local form validation does not merge patches or authorize
posting. Server ownership, verified-email/profile prerequisites, current account
eligibility, category lookup, conflicting configuration dates and supported IANA
zone lookup remain authoritative. Zone input checks syntax only.

These validators do not provide screens, presentation state, auth orchestration,
recurrence generation or interest calculation. Run their common tests through
the [shared unit-test commands](#shared-unit-tests); UI and live backend behavior
require separate integration work.

## Shared onboarding coordination — P11.03

Android's first-account flow uses a minimal authenticated `GET /v1/me` profile
snapshot to supply the real owner to the existing financial repositories.
`AuthSession.authorized` keeps resource requests, due renewal and sign-out in the
same ordering domain. Resource cancellation/network failure does not discard
otherwise valid credentials; a server 401 removes local authority.

`OnboardingSession` coordinates verification, explicit zone confirmation and
reviewed creation. `PendingAccountStore` is a write-ahead journal, not an offline
draft API. Android encrypts the immutable command and key using Keystore AES-GCM,
with owner/origin binding and no-backup storage. Unknown writes are recovered with
the original key across process restarts and same-owner sign-ins, never replayed
for another authenticated owner. iOS compiles/tests the common logic but has no
native onboarding screen or journal adapter yet. See
[ADR-058](../docs/architecture/decisions/058-first-account-onboarding.md).

## Android income and expense recording — P11.05

Home, Transactions, and active account details open one income/expense form.
Account details preselect the account and currency; switching record type or
currency never silently removes an incompatible selection. Ordinary income
requires a normal account or explicit no-account assignment. Expenses can use
normal or savings accounts. Archived accounts are not offered for new activity.

Enter an exact positive decimal amount and an explicit `YYYY-MM-DD` business
date, no later than today in the confirmed profile time zone. Verified users
without a zone can explicitly confirm one without creating an account. Category
and description are optional; the read-only catalog offers the applicable kind.
Review shows the report/account effects before Save. Cancel keeps editing or
discards only unsubmitted changes; success returns to the originating context.
History and filters remain P11.06, not part of this recording surface.

The encrypted, owner/origin-bound transaction journal is separate from account
onboarding. It is a write-ahead record, not an offline draft or an automatic retry
queue. Unknown completion retains the exact command and permanent operation key;
reopening or signing in never posts automatically. Use **Retry original request**
to resolve it. A confirmed validation rejection returns to editing; storage,
network or ambiguous responses never imply success. Retried operations reach
server idempotency handling even if account eligibility has changed since posting.

Shared logic compiles for iOS, but the native recording UI and journal adapter
are Android-only. See [ADR-060](../docs/architecture/decisions/060-android-income-expense-recording.md)
and the [execution evidence](../docs/plans/active/2026-11-08-android-income-expense.md).

## Android runtime and backup policy

The native app now requires **Android 7.0/API 24** and targets API 37. Navigation
Compose is 2.10.2; the maintainer approved dropping API 23 support. Earlier
execution records describe their original, lower minimum, not current support.
Shared-library compilation settings are unchanged.

Backup remains disabled: legacy full backup is explicitly off, and Android 12+
cloud/device-transfer rules exclude credential-encrypted and device-encrypted
storage plus external files. Secure session/journal records stay in no-backup
storage. There is no configured cross-platform export destination.

Run `python3 tests/mobile/test_android_policy.py` from the repository root for
policy regressions, then the existing Android build/lint and isolated emulator
suite. Platform/OEM backup transports and minimum-API physical devices need
separate verification; do not infer those results from lint.
See [ADR-061](../docs/architecture/decisions/061-android-runtime-and-backup-policy.md).

## Android transaction history — P11.06

`AuthenticatedHistoryReader` reuses the existing HTTP transaction/account/category
repositories inside `AuthSession.authorized`. Each read checks verified server
identity; one reader cannot mix owners. Stored dates do not require a profile
zone. This adapter never posts transactions or caches credentials or records.

The Android Transactions destination now reads posted/voided recorded events in
server order, with one row per transfer and both actual amounts/currencies.
Opening balances and transfers are explicitly not income/spending. No totals or
planned occurrences are synthesized. Record income/expense still opens the
existing reviewed form; returning refreshes history using the applied filters.

Filters are inclusive `YYYY-MM-DD` bounds, category, and all/specific/no account.
Blank dates impose no range. Choices include archived accounts and paginate
explicitly. Edit a draft, Apply to restart from page one, Cancel to keep applied
filters, or Reset then Apply to remove restrictions. All filters execute on the
server; there is no currency/type filter or incomplete-page local filtering.
Explicit refresh replaces loaded pages; next-page retry retains the original
cursor. Access denials clear financial state, and disposal ignores late reads.
Reads/drafts never enter saved instance state or persistent storage. See the
[P11.06 execution record](../docs/plans/active/2026-11-08-android-transaction-history.md).
