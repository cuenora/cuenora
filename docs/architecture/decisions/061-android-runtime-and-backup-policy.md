# ADR-061: Android API 24 minimum and explicit backup exclusions

- **Status:** Accepted
- **Date:** 2026-11-08

## Context and decision

The maintainer explicitly approved dropping Android 6.0/API 23 so the native app
can use Navigation Compose 2.10.2, whose minimum is API 24. Set the Android app's
minimum to 24 and target to 37, retaining compile SDK 37 and the existing AGP.
This does not change the shared library's own compilation minimum or implement
new product features. Historical foundation execution records retain their
original API 23 assumptions and verification limits.

Keep backup disabled. Declare `fullBackupContent="false"` for legacy Android and
explicit cloud/device-transfer exclusions for all nine Android backup storage
domains on Android 12+. Session and pending-operation journals remain encrypted
in no-backup storage; these rules do not move, delete or migrate existing data.
Do not configure an iOS cross-platform export destination. The app has no custom
backup agent. This is not a user data export feature, nor a promise of OEM
backup/restore certification.

## Consequences and verification

Android 6.0 devices can no longer install new native-app builds. Target 37 opts
into platform behavior changes, so compile/lint alone is insufficient; run the
existing navigation/authentication/financial instrumentation suite on API 37.
No physical-device or API 24 runtime result is implied by an API 37 emulator.

`python3 tests/mobile/test_android_policy.py` pins the approved SDK/dependency and
checks manifest references plus full cloud/device exclusions. It first failed
on the old policy; build, lint and runtime evidence are reported separately.
No lint suppressions or dependency manifest overrides are introduced.

## References

- [Navigation releases](https://developer.android.com/jetpack/androidx/releases/navigation#2.10.0-rc01)
- [Android 17 target behavior](https://developer.android.com/about/versions/17/behavior-changes-17)
- [Backup rules and excluded storage](https://developer.android.com/identity/data/autobackup)

### Observed local evidence

- Policy tests: three tests passed after the old configuration failed RED.
- Android assemble/lint passed with zero lint warnings or errors.
- Full isolated API 37 instrumentation: 42 tests, zero failures/errors/skips.
- Independent check confirmed packaged SDK levels, Navigation Android AAR minimum,
  packaged backup exclusions and SDK parser root-path semantics. Four unsafe
  policy mutations were rejected by the tests. No blockers found.
- API 24 runtime, physical-device and OEM backup transport behavior remain
  unverified. This does not claim remote CI or publication of these local bytes.
