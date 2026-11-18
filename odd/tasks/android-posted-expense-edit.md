# Edit posted expenses on Android
Branch: feat/android-transaction-detail-fidelity; baseline: 750d54e plus existing uncommitted detail changes.
Delivery: one scoped unit; Android host tests and isolated emulator. No push or PR authorized.

## Specs
- S1: “Coenctalos entonces, y lo demás que falta para completarlo”. Connect Edit record from posted expense detail; edit amount, business date, category and description only. Keep currency and account fixed. Use Pen v3Azl entry point and native form controls.
- S2: Preserve owner authorization and recover uncertain outcomes using the original immutable patch and idempotency key. Do not add change history or change backend behavior.

## Tasks
- T1 [S1,S2] inline implementation; shared journal-backed mutation, Android form, tests and documentation. Verified; local changes only.
- T2 [S1,S2] independent verification of write recovery and ownership. Verified after one bounded correction.

## Log
L1: “Coenctalos entonces, y lo demás que falta para completarlo”
The maintainer previously answered “Sí” to the posted-expense scope above. Existing local detail and void work must be preserved.

L2: The shared test initially failed to compile because the recovery session did not exist. Shared Android host suite and debug/test APK builds now pass. The detail entry point and review are wired. The Keystore journal retains the original exact patch/key and scopes it to backend and verified owner.
Risk: item 2 (owner authentication and encrypted write-ahead storage); independent verification required.

L3: Independent recheck met S1/S2 after correcting orphan AtomicFile recovery and terminal transaction_voided rejection. Shared Android host suite and debug/test builds passed after corrections. Emulator edit, void and encrypted journal recovery: 3/3. Repository and whitespace validation passed. RDD globally off. Worktree contains earlier uncommitted detail/filter work; no commit, push, PR or Samsung update in this task.
