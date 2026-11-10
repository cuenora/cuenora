# Android native client foundation

P10.02 introduced the integration host. P11.01 adds native Compose navigation,
Cuenora light/dark typography and colors, and reusable base controls. Four
unwired destination slots are explicit about their scope. The component preview
uses synthetic examples only. P11.02 adds native registration/sign-in through shared
repositories, secure session restoration/renewal and sign-out. The production
Activity gates these destinations behind authentication. P11.03 connects Home
to first-account onboarding. P11.04 connects Accounts to a paginated list and
read-only details; Transactions and Insights remain pending. HTTPS transport uses the Internet permission. See the [Android design handoff](../../docs/product/android-foundation/README.md).

## Build and check

From `mobile/`, using the JBR/SDK configuration in the [mobile setup](../README.md):

```bash
./gradlew --no-daemon :androidApp:assembleDebug :androidApp:lintDebug
```

The APK is under `androidApp/build/outputs/apk/debug/`. Android's default debug
signing makes this development APK installable; release signing is not configured.
For isolated checks, set `ANDROID_USER_HOME` to an external temporary directory
so the build creates a disposable debug keystore instead of using local credentials.

The development identifier `org.cuenora.android`, minimum SDK 23 and target SDK 36
are provisional, not reserved store identifiers or a final app support policy.
Compilation uses SDK 37 because the selected AndroidX libraries require it;
this does not change the target or minimum SDK.
Jetpack Compose libraries use BOM 2026.09.00; Activity Compose is pinned to 1.13.0.
The Compose compiler follows Kotlin 2.4.21 independently of the BOM.

## Authentication and session lifecycle

The configured backend remains `BackendConfig.BASE_URL`. Launching the app reads
only local secure storage when no session exists; sign-in/registration make real
requests when explicitly submitted. Do not use personal credentials or assume a
development build targets an isolated API. This task's automated checks use
synthetic repositories/MockEngine, not the hosted backend.

Registration acceptance does not auto-login or promise account/email creation.
After signing in, `AndroidSessionStore` persists the pair before the shell opens.
Restoration rotates the stored pair; foreground checks renew before expiry.
Rotation removes the previous saved pair before sending its single-use token,
so interrupted/offline renewal requires signing in again rather than replaying it.
A secure-storage failure blocks access and offers explicit local-clear retry.

Sign-out removes local credentials first, then requests current-session server
revocation. The app explicitly says when remote sign-out was not confirmed.
Passwords are never saved in Activity state, are hidden and cleared on submission
or backgrounding, and the production window prevents screenshots. See
[ADR-057](../../docs/architecture/decisions/057-native-authentication-lifecycle.md).
P11.03 adds first-account onboarding below. Password recovery and global sign-out
remain outside the current native journey.

## First-account onboarding — P11.03

Home reads authenticated `GET /v1/me` before using the shared account repository.
The configured backend must support that additive profile read. No owner identity,
verification state or saved time zone is inferred from opaque login tokens.

1. A pending user requests/resends verification, then confirms an emailed token
   here or uses the existing browser link with same-account login and checks again.
   Acceptance does not promise delivery. Tokens are hidden, never saved and cleared
   on submission/background. Android app-link interception is not implemented.
2. A verified owner with no accounts can add a normal/savings account in SEK,
   EUR or USD. Existing owners are not forced through first-account setup.
3. A saved profile zone is preserved. For a missing zone, the device zone is only
   a suggestion: explicitly confirm an IANA identifier before a dated opening.
   Exact signed opening amounts and business dates are reviewed before saving;
   openings are starting movements, not ordinary income.
4. A random creation key and immutable command are encrypted in a separate
   owner/origin-bound no-backup journal before POST. Unknown outcomes offer only
   retry of the original operation. Restart/sign-out does not turn an unknown
   outcome into a new creation; another signed-in owner cannot replay it.

Cancel/back offers Keep editing or Discard before financial submission. A loading
failure is never rendered as an empty account list. Secure-journal failure blocks
creation, with no plaintext fallback or offline-write promise. Accountless entry
forms remain P11.05; onboarding does not require an account for those future accountless forms. See [ADR-058](../../docs/architecture/decisions/058-first-account-onboarding.md)
and the [execution plan](../../docs/plans/active/2026-11-08-android-onboarding.md).

All automated checks use synthetic repositories and an isolated emulator/database,
not the configured hosted backend or a personal device.

## Instrumented foundation and authentication tests

Start an isolated Android emulator or attach an authorized test device. Run:

```bash
./gradlew --no-daemon :androidApp:connectedDebugAndroidTest
```

Tests cover destination selection/restoration/Back, responsive navigation,
light/dark brand pairs, invalid input, unavailable/busy controls and safe dialog
focus. Authentication tests cover sign-in/sign-out, generic registration,
restoration, safe credential/network failures and blocked secure storage using
fakes. A debug-only FoundationActivity isolates navigation/component tests from
production authentication and network access; it is absent from release builds.
These are app UI tests, not shared-library or live financial end-to-end tests.
JUnit reports are under `build/outputs/androidTest-results/connected/debug/`.
If the default 512MiB Gradle heap thrashes while dexing instrumentation, pass
`'-Dorg.gradle.jvmargs=-Xmx3g -XX:MaxMetaspaceSize=1g'` locally; no global Gradle
configuration change is needed. Navigation 2.9.7 retains API 23; newer 2.10 requires API 24. The corresponding
newer-version lint warning remains visible. Espresso 3.7.0 is explicit because the older
transitive version calls a removed InputManager method on the API 37 emulator.

## Earlier integration evidence


On 2026-11-06, `assembleDebug` and `lintDebug` passed with JBR 25.0.3. The first
SDK 36 build failed AndroidX AAR metadata validation; changing only `compileSdk`
to 37 resolved it without suppression. APK inspection confirmed the label,
development ID, SDK bounds and a call to the shared marker getter.

Lint reports no errors and three unsuppressed scaffold warnings: old target API,
missing data-extraction rules and missing application icon. Release policy,
backup behavior and branded assets are not established by this integration host.
That earlier integration check did not establish UI runtime behavior. Current
foundation verification is recorded in the linked P11.01 execution record.


## Account list and details — P11.04

Accounts reads paginated server records using the current authorized session.
Each page checks the verified server profile; a reader is bound to one owner
for its signed-in UI lifetime. No bearer or account record is saved by this UI.
Archived accounts remain visible alongside active normal and savings accounts.

Select an account to see its name, type, currency, status and exact posted balance.
That balance is from the last successful read, not a live bank balance, forecast
or cross-currency total. Back keeps the loaded pages and list position. Refresh
accounts explicitly to read new values; refresh discards the prior snapshot rather
than presenting stale data as current. Leaving the signed-in subtree clears the
memory-only browser and prevents a late read from repopulating it.

Loading, confirmed empty and failed reads are distinct. A failed additional page
keeps existing rows and retries the same cursor; failed/denied fresh reads never
produce invented zero balances. An empty list directs first-account setup to Home.
These screens are read-only: activity, pending payments, management and additional
creation controls are not part of P11.04. Activity remains P11.06.

See [ADR-059](../../docs/architecture/decisions/059-android-account-browsing.md)
and the [execution record](../../docs/plans/active/2026-11-08-android-account-browsing.md).

## Recurring payments — P11.08

Home's **Recurring payments** opens rules and occurrences through the authenticated
shared recurrence session. The backend must include the additive [listing
APIs](../../docs/product/recurrence-listing.md). The initial occurrence view is
planned; filters also expose confirmed/skipped history. Load more preserves
server ordering. A failed read is not presented as an empty list.

Create an expense rule with an explicit IANA zone and weekly/monthly/quarterly/
annual cadence. Rule edits apply only to future generation. Pause/resume preserve
existing occurrences and the original calendar; resume does not backfill paused
periods. Generation remains a separate server job, not an Android timer.

Review each actual payment independently: actual amount, currency, business date,
optional account/category and description. Reviewing or choosing **Not paid yet**
never posts. Only explicit confirmation creates one expense; an accountless
payment affects expense reports without an account movement. This records a
payment already made, not a bank instruction. Skip is terminal and planning-only;
postpone changes one due date strictly forward without shifting its scheduled
identity or rule calendar.

Keyed operations use a separate encrypted no-backup owner/backend-bound journal.
Unknown outcomes permit only an explicit original-command/key retry, including
after restart; navigation never discards that journal. Rule edit/pause/resume
have no operation key: after an uncertain outcome, refresh and review current
server state rather than automatically replaying. Existing account/transaction
journals remain unchanged. No offline-write queue, physical-device smoke or
hosted-backend verification is implied by local tests.

## Savings terms, forecasts and actual interest (P11.09)

From a savings account's details, choose **Savings terms and interest**.
Terms and nominal annual rates are independent immutable effective-date histories;
load additional pages explicitly. Add a new date/version rather than editing or
deleting history. Frequency and projected capitalization require explicit choices;
the review explains the fixed daily-close, actual-calendar-year and half-even
calculation policies. Configuration never posts money.

Forecast dates start empty and must be chosen explicitly (inclusive 1–366 days,
starting today or later in the confirmed profile zone). Results show the posted
snapshot date/balance, rate segments, exact fractional minor-unit accrual and
arbitrary-precision projected payouts. Open periods have no rounded payout.
The balance is held flat except projected capitalization; prior accrual is excluded.
A partial period must not be treated as the next complete bank payout.

For an actual receipt, explicitly select the active owned account that received
it (normal or savings), enter the actual amount/date and review before confirming.
The account's currency is used; terms and a matching forecast are not required.
Only the receiving account is credited; no second credit is added to savings.
Actual receipts are realized interest income, not ordinary savings funding.

Requests are persisted before sending in a separate owner/origin-bound encrypted
no-backup savings journal. Unknown outcomes offer **Retry original request** only;
opening, refresh, navigation and sign-in never replay automatically. Keyed receipts
retain the original command/key; configuration retries retain the original natural
account/date slot. Leaving preserves unresolved intent. A definite configuration
validation/conflict response permits reviewing corrected input; an unconfirmed
receipt is never replaced. Secure-storage failure blocks new writes.

Local checks use the shared Android-host/iOS simulator suites, separate artifact
build and standalone lint commands from mobile setup, plus `SavingsTest` and
`AndroidSavingsJournalTest` in an explicitly owned disposable emulator. A build or
native-test compilation alone is not runtime proof. Hosted/physical-device smoke,
minimum-supported-API and full accessibility certification remain separate checks.
