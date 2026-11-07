# ADR-053: Native secure session storage

- **Status:** Accepted
- **Date:** 2026-11-07
- **Roadmap:** P10.07

## Context

Refresh rotates an access/refresh pair. ADR-052 HTTP transport does not retain
credentials. Both mobile platforms need storage without introducing authentication
orchestration, repositories, financial-domain dependencies or a plaintext fallback.

## Decision

`SessionStore` exposes suspend read/replace/clear. Read returns null only when no
record exists; failures throw a redacted `SessionStoreException` with Unavailable
or Corrupt reason. Swift-facing methods declare these exceptions. Credentials are
not data classes and redact display; raw values are exposed only for callers that
must build authentication requests. No logs or raw exception causes are retained.

Store one bounded versioned record containing both nonempty opaque ASCII tokens,
not separately persisted fields. An internal codec is shared by both platforms.
Default namespace is BackendConfig.BASE_URL; custom backends must supply matching
namespaces. Trailing slashes are normalized; other namespace values remain opaque.
Changing the configured backend therefore does not reuse the prior backend's pair.

Android protects a random AES-256-GCM key with AndroidKeyStore and stores only
authenticated ciphertext in app-private no-backup storage. Every replacement uses
a provider-generated IV; namespace/version bind authenticated data. Writes replace
the whole record atomically. Missing keys or tampered records never yield partial
credentials. No deprecated encrypted-preferences dependency is introduced.

iOS uses one generic-password Keychain item, scoped by service and namespace,
with WhenUnlockedThisDeviceOnly accessibility and synchronization disabled. Update
the existing item or add when absent; do not delete before replacement. Do not
modify other namespaces or unrelated Keychain items.

The Core Foundation dictionary boundary inserts exact `kCFBooleanTrue`/`False`
values for Security flags. Generic Kotlin/NSNumber boxing produced CFNumber and
caused `SecItemCopyMatching` to reject the read query with errSecParam (-50).
NSNumber construction alone was insufficient. A deterministic Native test checks
CFBoolean type/value; app-hosted tests exercise the real public storage API.

All store instances serialize operations within the process and perform blocking
native work off the caller thread. This does not serialize a complete HTTP refresh
transaction: orchestration must still coordinate network refresh and replacement.
Cancellation or a native failure can leave a committed operation ambiguous; callers
must handle the typed outcome, not blindly retry refresh or assume local logout.

## Limits and verification

No cross-process coordination, biometric requirement, universal hardware-backed
guarantee, secret-memory zeroization or uninstall/reinstall policy is promised.
When locked, Keychain may be unavailable rather than empty. Android key-use policy
does not claim identical lock-state restrictions on all supported SDKs. Backup/sync
exclusion does not protect a compromised process using its own credentials.

Common tests verify codec/error behavior; Android instrumentation and iOS
app-hosted XCTest exercise native storage. The command-line Native runner could
not use Keychain, even after the query fix, while all six app-host cases passed.
The service cases are therefore located in the existing iOS application's test
target, not ignored in common tests. Gradle allTests includes the pure Native
bridge regression, not the Keychain service tests; release verification needs both
Gradle and XCTest commands. Build-only and JVM mocks cannot establish native
behavior. XCTest resolves the single shared runtime from the app with BUNDLE_LOADER
and disables autolinking a second static copy. Test minimum iOS17 matches the SDK's
XCTest libraries; app/shared minimum iOS15 is unchanged. No development-team
credential or custom entitlement is required for local simulator ad-hoc tests.

Sources: [Android Keystore](https://developer.android.com/privacy-and-security/keystore),
[Apple item accessibility](https://developer.apple.com/documentation/security/restricting-keychain-item-accessibility),
[Security return-data flag](https://developer.apple.com/documentation/security/ksecreturndata),
[Keychain update/delete](https://developer.apple.com/documentation/security/updating-and-deleting-keychain-items).
