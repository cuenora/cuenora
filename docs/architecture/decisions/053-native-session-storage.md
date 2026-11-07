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

Common tests verify codec/error behavior; Android instrumentation and iOS runtime
tests must exercise native storage. Build-only and JVM mocks cannot establish
Keystore/Keychain behavior. Native access-control failures are not waived.

Sources: [Android Keystore](https://developer.android.com/privacy-and-security/keystore),
[Apple item accessibility](https://developer.apple.com/documentation/security/restricting-keychain-item-accessibility),
[Keychain update/delete](https://developer.apple.com/documentation/security/updating-and-deleting-keychain-items).
