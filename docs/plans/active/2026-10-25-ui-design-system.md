# Execution plan: Initial UI design system

- **Status:** Completed locally; repository integration pending
- **Date:** 2026-10-25
- **Roadmap task ID(s):** P01.10
- **Tracking issue / PR:** [PR #41](https://github.com/cuenora/cuenora/pull/41); no tracking issue
- **Owner:** Agent

## Goal and user-visible outcome

Define initial semantic design tokens, reusable UI components and interface
states in the existing Pen document, extending P01.09 without implementing UI.

## Existing state and evidence

- P01.09 provides 11 blue shades, Bodoni Moda/IBM Plex Mono and three references.
- Pen has two reusable components: primary action `Vuzih`, navigation `t59Pc`.
- P01.07/P01.08 grayscale boards already define empty, error, validation,
  unknown-outcome, planned and confirmed states. P01.05 owns recovery semantics.
- Existing `.pen`, plan links and P01.09 assets are uncommitted; preserve them.

## In scope

- Semantic colors, typography, spacing, sizing, borders, focus and motion rules.
- Reusable action, input, selection, amount, row, notice and dialog specimens.
- Read, write, recovery and financial-meaning states with accessible cues.
- Narrow/large-text examples, token JSON snapshot, exports and handoff guide.

## Out of scope

- Runtime UI, full platform themes, API recovery endpoints and financial rules.
- Restyling or replacing existing references. Publication is a separately authorized
  delivery step, not additional design scope.

## Constraints and invariants

- English artifacts; keep P01.09 primitives intact and namespace new variables.
- Preserve accountless recording, distinct currencies and planned/realized data.
- Never turn unknown reads into zero balances or uncertain writes into success.
- Posted records use audited correction, not casual deletion.
- Static accessibility checks do not certify native behavior or screen readers.

## Steps

- [x] 1. Define and illustrate semantic tokens.
- [x] 2. Build reusable components and their interaction variants.
- [x] 3. Apply components to state and adaptive reading specimens.
- [x] 4. Audit, export and document; verify native save before final completion.

## Acceptance criteria

- [x] Token names/values and light/dark roles are explicit and exported.
- [x] Core controls cover applicable default, focus, pressed, disabled and error states.
- [x] Read/write/recovery states preserve the existing product semantics.
- [x] Contrast, control boundaries and resolved layout are checked.
- [x] Scope limitations and platform implementation checks are documented.

## Verification

| Command or scenario | Expected result | Observed result | Status |
| --- | --- | --- | --- |
| Pen token, component and state readback | Named variables and reusable components | 76 tokens, 17 reusable components (15 new), 15 state specimens | Passed |
| Contrast and resolved-layout audit | Text ≥4.5:1, essential cues ≥3:1, no clipping | 262 texts, 55 icons, 21 boundary samples; no failures or clipping; 44 JSON color-pair checks passed | Passed |
| Narrow and 200% text specimens | Sample copy and amounts retained | Both 320-wide samples checked; full sample amounts retained, controls grow | Passed (static only) |
| Export/token snapshot consistency | Saved artifacts match Pen variables | Four PNGs, four-page PDF, JSON exactly matches subsequent Pen token readback | Passed |
| Prior-artifact preservation | Existing references and palette retained | All 34 prior roots present; 11 P01.09 blue values unchanged | Passed |
| Native save evidence | File updated after final canvas edits | `.pen` timestamp 19:50:52 +01:00, after final exports at 19:49:50; later Pen readback without mutations | Passed |
| `python3 scripts/check_repository.py` | Pass | 155 Git-visible files examined | Passed |
| `git diff --check` | Pass | No whitespace errors | Passed |

## Risks, assumptions, and decisions

- Inline design work: one initial system deliverable, resumable from named Pen
  boards and guide. No production code or existing consumer contract changes.
- Static design has no meaningful functional RED/GREEN cycle; inspect structure,
  contrast, references and recovery copy instead.
- Design dimensions map to scalable native units later. Motion is optional,
  non-blocking and removed under reduced-motion settings.

## Completion notes

Created boards `TCCsq`, `VsuWv`, `PsG2u` and `WTGzK` in the existing Pen document;
added `docs/product/ui-design-system.md` and `docs/product/design-system/` exports
and token snapshot. Updated the plan index/master and P01.09 handoff links.

Light/dark roles, feedback accents, control targets and recovery rules are design
definitions, not runtime implementation. Native focus, screen readers, maximum
amount/localization cases, responsive clients and reduced motion remain client
implementation checks, not open P01.10 design tasks.

The maintainer authorized one PR for P01.09, P01.10 and `DESIGN.md`, including
an explicit size exception for the shared Pen source. Use the existing
`documentation` label; do not create issues or new labels. Merge is not authorized.
Published design commit `83fa68b` on `docs/p01-09-10-design-system` in PR #41
against `main`. The initial commit contains 25 files, 13,757 additions and two
deletions; 12,383 additions belong to the shared Pen source. The native assessment
returned `medium`, `review_due=false` (`under_budget`), with `.atl/` excluded;
no receipt review was started. Font-notice whitespace and line endings were
normalized before commit without changing license wording. Publication metadata
is recorded separately from the design commit. Runtime checks were not needed
for the static artifacts; GitHub's existing CI still runs on the PR.
Preserve `.atl/`. Keep this plan under `active/` until its PR is merged, then
archive it under `completed/` according to repository policy.
