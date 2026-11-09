# Execution plan: Android recurrence and payment confirmation

- **Status:** Completed locally; publication pending
- **Date:** 2026-11-09
- **Roadmap task ID(s):** P11.08
- **Tracking issue / PR:** Not created; one future size-exception PR approved
- **Owner:** Maintainer and agent

## Goal and user-visible outcome
Create and manage recurring expense plans and explicitly record actual payments in Android. Planned, skipped and postponed items never create financial postings.

## Existing state and evidence
Backend mutations exist; listing endpoints and native recurrence workflows are missing. Shared validation supports recurrence drafts. See the [task specification](../../../odd/tasks/android-recurrence.md) for exact scope, decisions and evidence.

## In scope
Rule create/list/edit/pause/resume; occurrence list/review/confirm/skip/postpone; additive owner-scoped paginated backend reads; shared adapters, secure retry recovery, native integration and tests.

## Out of scope
New recurrence generation policy or job, schema/dependency changes, bank payment execution, iOS UI, hosted/physical-device operations and publication.

## Constraints and invariants
Existing recurrence lifecycle, rules, confirmation and occurrence-action contracts are authoritative. Exact money; explicit rule zone and actual payment review. Serialize lifecycle actions; preserve keyed commands before sending and never automatically replay unknown writes. Non-keyed uncertainty requires refresh/review. No changes to existing journal formats.

## Steps
- [x] Backend listing contracts, implementation and isolated tests.
- [x] Shared lifecycle, HTTP adapters, secure journal and deterministic tests.
- [x] Android workflows, native tests and user documentation.
- [x] Own checks, independent risk verification, local commits and closure.

## Acceptance criteria
- [x] S1-S5 in the task specification are covered with observed checks.
- [x] Foreign reads and actions do not expose other owners' resources.
- [x] Rule actions preserve existing occurrences; occurrence actions preserve planning/posting distinctions.
- [x] Unknown keyed outcomes retain their original intent/key; refresh never automatically retries.
- [x] Existing financial/session workflows remain verified.

## Verification
| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Cargo and isolated PostgreSQL recurrence reads | Owner-scoped bounded pages; writes unchanged | Format/Clippy passed; workspace191 passed,254 explicitly ignored; disposable listing1 passed; existing mutation harness passed baseline/candidate | Passed |
| Shared Android-host/iOS simulator suites | All tests pass | Final full181/182 passed, no failures/errors/skips; adopted stronger retirement probes2 passed separately | Passed |
| App builds/lint and explicit isolated-emulator tests | All checks pass | AAR/two frameworks/APKs built; standalone lint passed; API37 host-GPU native104 and encrypted journal1 passed | Passed |
| Independent items2/4 verifier | No change-caused blocker | B1 retirement race reproduced, corrected and original-blocker recheck2 passed; existing shared flow differential66/66 matched | Passed |
| Repository validator/diff check | Pass | Parent repository validation, diff check and APK-build spot check passed | Passed |

## Risks, assumptions, and decisions
Risk: items2/4 (authenticated financial authority and asynchronous write/recovery ordering). Additive list APIs do not change existing consumed contracts. One PR size exception approved. RDD globally off. No physical-device or remote operations.

## Completion notes
P11.08 is implemented and locally verified; no Phase11 completion claim. The
independent verifier reproduced a POST starting after presentation retirement
when durable journal saving suspended. The bounded correction rechecks active
owner after storage and before HTTP; regression tests prove zero late POSTs,
unchanged durable command and explicit fresh-session retry with the original
key/body. The original blocker recheck passed; no new sweep was launched.

Final mobile production source was verified with full181/182 shared tests,
native104 plus encrypted journal1, APK/AAR/two frameworks and standalone lint.
Independent stronger assertions were adopted as regression tests and passed2/2;
these changed tests only after full-suite proof, not production code. Parent APK
build spot check passed. Existing shared authentication/account/recording/history
checks matched baseline187d001 and candidate66 cases each.

### Check limitations and environmental failures

Ordinary Cargo's254 ignored PostgreSQL tests are not claimed as executed. The
scoped disposable recurrence listing harness passed; writer also checked existing
recurrence mutations on baseline and candidate. No schema or dependency changes.

Lint initially exhausted384MiB metaspace; the documented per-command heap override
(`-Xmx3g -XX:MaxMetaspaceSize=1g -XX:-HeapDumpOnOutOfMemoryError`) passed without
source configuration changes. A combined lint/shared-test Gradle invocation hit
the previously baseline-reproduced generated-fixture dependency defect documented
in the foundation record. Separate shared-test, artifact and lint invocations
passed. That baseline defect is not fixed or suppressed by this feature.
Existing Kotlin/Native cached-object warnings remain visible.

The final source/config/resource manifest matched743 inputs before/after native
checks (SHA256 `72a0e426ffbd178ddef23383024a2d29f17838339958acdf9896ba9876ceee11`).
Native evidence uses an owned qemu1/API37 `Cuenora_T2` instance, explicit serial5598,
GPUhost, direct serial-bound installation/instrumentation and confirmed shutdown.
An earlier detached launch failed before boot/install; the supervised launch
retained the process through its full lifecycle. No connected Gradle, device
discovery, physical-device or hosted API operation was performed.

No minimum-supported-API runtime, real-backend mobile smoke, production readiness
or full accessibility certification is implied. Backend deployment must include
the two new reads before this Android feature is used against it. Publishing and
PR creation remain separate maintainer decisions; one future size-exception PR
is approved. Raw local logs retained under `/tmp/cuenora-recurrence-verification.lSNIWg`;
large scratch copies are removed by the parent after retaining bounded probe/log evidence.

### Local delivery

- `c4520c4` — additive backend reads, contract and isolated coverage.
- `6d32b67` — shared reviewed lifecycle, encrypted recovery and regression tests.
- `3f12b1c` — Android screens, native tests and user documentation.

These coherent units total3098 authored changed lines before the passive closure
record; the maintainer approved one future size-exception PR. RDD remains globally
off (`disabled/unmanaged`), without review lifecycle or publication. After commits,
parent readback matched333 executable/production inputs to checked bytes; the
only later test change adopts the independent stronger original-key/body assertions.
Named scratch copies were removed after retaining bounded evidence; one early
worker-created temporary directory lacked a recorded path and was not blindly
deleted. No scratch or generated heap dump remains in the worktree.
