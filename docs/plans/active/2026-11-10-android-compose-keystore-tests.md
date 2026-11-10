# Execution record: Compose and Keystore regression tests

- **Status:** Locally verified; publication pending
- **Date:** 2026-11-10
- **Roadmap task:** P11.12
- **Tracking issue / PR:** Not created

## Outcome and existing coverage

Added two focused regression tests rather than duplicating existing navigation,
form, authentication, ciphertext, tampering, namespace and concurrent-pair tests.
No production code, dependencies or security policy changed.

- `ComponentsTest.errorNoticeExposesHeadingAndRecoveryMessage` asserts that an
  error notice exposes its title as an accessibility heading and displays its
  recovery message.
- `AndroidSessionStoreTest.clearDeletesKeystoreAliasAndFreshInstanceCanSaveAgain`
  uses the actual Android Keystore. It checks alias presence after saving,
  alias and record absence after clearing, an empty fresh store, and a complete
  new credential pair readable by another instance after saving again. A finally
  block clears the synthetic test namespace.

## Verification

These are coverage additions for existing behavior, not bug fixes: no failing
production behavior or RED result is claimed. Both new tests passed against the
unchanged implementation. No command/API behavior changed, so a differential
production-output check was not applicable.

Gradle commands below ran from `mobile/`, with `--offline --no-daemon`.
A fresh disposable API37 emulator was checked for `ro.kernel.qemu=1`; an isolated
ADB server on port5039 exposed only that emulator. The physical Samsung was not
used. Connected tasks were run against this isolated server, not the ordinary
server with the physical device attached.

| Check | Observed result |
| --- | --- |
| `:androidApp:assembleDebug :androidApp:assembleDebugAndroidTest :shared:assembleAndroidDeviceTest :androidApp:lintDebug` | Passed |
| Focused new Compose test | 1 passed |
| Focused new Keystore test | 1 passed |
| `:androidApp:connectedDebugAndroidTest` | 118 tests, zero failures/errors/skips |
| `:shared:connectedAndroidDeviceTest` | 8 tests, zero failures/errors/skips |
| Parent readback of both JUnit XML reports | Counts and zero failures/errors/skips confirmed |

The emulator and isolated server were stopped after the checks. Native review was
not started because the user-owned mode was globally disabled.

## Limits and follow-up

No minimum-API, physical-device, hosted CI, backend or iOS rerun is implied.
Keystore deletion assertions prove the platform alias lifecycle, not forensic
physical erasure. The Compose test pins semantics, not actual spoken output;
maintainer-reported TalkBack checks remain in the P11.11 record.
Changes are uncommitted and unpublished. Prior P11.11 documentation was preserved.

Risk: none (test-only additions; production behavior unchanged).
