# Initial UI design system

**Roadmap:** P01.10 · **Status:** Initial design system completed · **Date:** 2026-10-25

Cuenora now has **76 named design tokens**, **17 reusable components** including
the two P01.09 sources, and **15 interface-state specimens**. The system extends
the [palette and typography](color-and-typography.md), preserving the existing
wireframes and three high-fidelity references.

This is a design definition and handoff, not implemented mobile UI or a complete
application accessibility certification. The native document's timestamp advanced
to 19:50:52 +01:00 after the final 19:49:50 exports. Subsequent Pen readback
confirmed the token values and boards without further canvas mutations.

## Review the system

Open the [four-page PDF](design-system/export.pdf) or individual boards:

| Board | PNG | Pen frame |
| --- | --- | --- |
| Semantic tokens | [Foundations](design-system/TCCsq.png) | `TCCsq` |
| Reusable components and interaction variants | [Components](design-system/VsuWv.png) | `VsuWv` |
| Interface states and recovery | [States](design-system/PsG2u.png) | `PsG2u` |
| Narrow, large-text and dark-control specimens | [Adaptive reading](design-system/WTGzK.png) | `WTGzK` |

Editable layers are in [cuenora-design.pen](../cuenora-design.pen), under
`P01.10`. The [token JSON snapshot](design-system/tokens.json) mirrors Pen's
`p0110-*` variables. It is a Cuenora design-reference format, not a production
configuration or a claim of interoperability with a standard token format.

## What was already present

P01.09 provided the blue ramp, the two font families, three high-fidelity mobile
references and reusable primary-action/navigation components. P01.07 and P01.08
provided low-fidelity state copy. P01.10 adds purpose-based tokens, common
control definitions, interaction rules, reusable feedback and adaptive examples.
The [existing recovery paths](navigation-and-flows.md#shared-alternate-and-recovery-paths)
remain authoritative; no new financial or API recovery behavior is introduced.

## Token contract

Use semantic roles in new components. Keep `p0109-*` primitives and historical
references intact; do not substitute arbitrary palette shades for a purpose.
Light and dark roles are separate variable names, not a working theme switch.

| Role | Light | Dark |
| --- | --- | --- |
| Canvas | `#FFFFFF` | `#040B16` |
| Surface | `#FFFFFF` | `#0B1E3D` |
| Muted surface | `#E9F0FB` | `#143770` |
| Primary text | `#0B1E3D` | `#E9F0FB` |
| Secondary text | `#18448B` | `#9BBBED` |
| Essential control boundary | `#1F57B2` | `#74A0E7` |
| Decorative separator | `#C2D5F4` | `#18448B` |
| Primary action: default / hover / pressed | `#2466D1` / `#1F57B2` / `#18448B` | `#9BBBED` / `#C2D5F4` / `#74A0E7` |
| Primary action label | White | `#040B16` |
| Underlined link | `#1F57B2` | `#9BBBED` |
| Focus indicator | `#18448B` | `#C2D5F4` |
| Disabled surface / text | `#E9F0FB` / `#18448B` | `#143770` / `#9BBBED` |
| Selected surface / text | `#C2D5F4` / `#0B1E3D` | `#143770` / `#E9F0FB` |

Decorative separators do not identify essential controls. Focus uses a 2-unit
outline with a 2-unit surface-colored gap, so it remains distinct from the
control boundary. Recalculate contrast if a surface, opacity or adjacent color
changes.

### Process feedback

Three restrained accents supplement the blue identity for process feedback.
They do not assign moral meaning to income/spending or replace explicit labels.
Notices retain their own opaque surface even on a dark screen.

| Tone | Foreground / background | Contrast | Required cue |
| --- | --- | ---: | --- |
| Information | `#18448B` / `#E9F0FB` | 8.18:1 | Information icon and explanation |
| Confirmed success | `#16633F` / `#E9F6EE` | 6.53:1 | Check icon and confirmed result |
| Attention / uncertain outcome | `#7B4A00` / `#FFF3DA` | 6.76:1 | Warning icon and next-step explanation |
| Error / unavailable read | `#9B2440` / `#FFF0F3` | 6.99:1 | Error icon or explicit field-error label |

### Typography, geometry and motion

| Group | Definition |
| --- | --- |
| Font families | Bodoni Moda for short display/headings; IBM Plex Mono for UI and amounts |
| Sizes | Display 44; heading 32; amount/section 24; notice 18; body 16; metadata 14; compact navigation 13 |
| Specimen-only title | 52, for reference-board titles rather than product UI |
| Weights / leading | 400 regular, 500 medium; body 1.5, headings 1.2 |
| Spacing | 4, 8, 12, 16, 20, 24, 32, 48 |
| Shape | Control radius 10; card 12; pill 999 when a true capsule is needed |
| Controls | Minimum target 48; baseline control 56; grow with text rather than fixing height |
| Icons / boundaries | Icon 24; default border 1; focus width 2 and gap 2 |
| Motion | Optional transitions 120/180 ms; remove optional movement under reduced motion; no flashing |

Length tokens are design pixels, later mapped to scalable platform units;
duration tokens are milliseconds, and line heights are font-size multipliers.
Font weights are numeric reference values, mapped to each renderer's supported
representation. The 48-unit target is a project choice, not a statement that
WCAG's minimum is 48. Native font scaling, motion and accessibility settings are
requirements for implementation; the canvas does not execute them.

## Component inventory

| Component | Pen ID | Definition |
| --- | --- | --- |
| Primary action, reused | `Vuzih` | Default, hover, pressed, focus, disabled and busy specimens |
| Four-destination navigation, reused | `t59Pc` | Existing destinations, explicit current-item styling |
| Secondary action | `hmtcM` | Cancel/back or alternate action |
| Destructive action | `GkyVM` | Discard unsubmitted edits; not deletion of posted records |
| Text link | `WcnFO` | Underline plus action cue |
| Amount field | `MUyL2` | Persistent label, explicit currency, preserved input and helper/error |
| Selector / date trigger | `KK78I` | Persistent label, disclosed value, currency error and date variants |
| Checkbox | `W0lZCZ` | Labeled full-row target, checked/unchecked |
| Radio | `bqoIu` | Single-choice selection with dot and ring |
| Switch | `w6eOW` | On/off text and thumb position; preference only |
| Amount display | `gaUdS` | Meaning, currency and full signed value |
| Status badge | `amF1i` | Explicit planned/recorded meaning, not color alone |
| Account disclosure row | `s9cJ7r` | Owned-account name, type, recorded amount and disclosure cue |
| Notice | `nYJBH` | Icon, title and explanation; four feedback tones |
| Loading placeholder | `wAv3E` | Busy explanation and decorative placeholders; no fabricated amount |
| Discard-draft dialog | `q1P60` | Safe Keep editing action and explicit discard alternative |
| Large-text destination list | `yijzZ` | Same destinations when a four-item bar cannot fit |

### Shared interaction rules

- Default is actionable, not selected or successful. Hover is optional for
  pointer-capable clients. Pressed is transient, never proof of persistence.
- Keyboard focus uses the shared outline around the entire target. Focus must
  remain visible and have a logical order; focus is not conveyed by color alone.
- For secondary/link/selection controls, hover/pressed use the selected surface
  while retaining the role's text and boundary. Selection still has an underline,
  check, dot, position, outline or explicit state label.
- Disabled controls do not activate; expose the disabled state natively and show
  the reason nearby. Do not rely on opacity. A busy submission also prevents
  duplicate activation and uses progress wording, not a success message.
- Invalid fields retain input and expose a specific error associated with the
  persistent label. Validation applies no partial financial effect.
- Dialogs initially focus the safe action, contain focus while open, support
  dismissal where safe and return focus to their invoker. Those behaviors remain
  native implementation work, not functionality of these static components.

The board illustrates representative interaction variants and defines the
shared rules for all controls; it is not an exhaustive rendering of every
component/state Cartesian product.

## Interface-state contract

| State | Required meaning and recovery |
| --- | --- |
| Loading | Explain busy state; unknown data is not zero |
| No accounts | Offer account creation and retain accountless recording |
| Filtered no results | Explain that records remain; edit/reset filters |
| Read failure | Current data unavailable; retry the read, not a financial write |
| Validation | Preserve values; choose a same-currency account or leave an expense unassigned |
| Submission in progress | Resolve the original request; prevent duplicate submission |
| Confirmed write | Show the existing recorded expense; do not offer a second confirmation |
| Unknown write outcome | Neither failure nor success; resolve the original operation, never create another |
| Session expired | Request sign-in and mask sensitive data; no silent resubmission or draft-persistence promise |
| Unsaved changes | Keep editing or discard unsubmitted changes; cancellation changes no records |
| Confirmed empty period | Real report zeros, not a failed read; omit percentages when spending is zero |
| Planned / estimated | Explicitly excluded from recorded balances until a real event is confirmed |
| Overdue occurrence | Still planned; a due date does not post spending |
| Unsupported estimate | No estimate, while tracking and actual interest recording remain available |
| Unavailable access | Do not reveal whether someone else's record exists; offer safe navigation |

Additional domain-empty states reuse the empty pattern with the existing
[P01.07](accounts-and-transactions-wireframes.md) and
[P01.08](recurrence-savings-insights-wireframes.md) copy: no transactions,
recurring rules, savings accounts or pending matches through a selected date.
Unknown-outcome status/retry controls remain owned by the later API/client
idempotency contract; this design intentionally does not invent a retry endpoint.
All example amounts are synthetic and the state panels are independent examples.

## Adaptive and accessibility handoff

The fourth board compares 320-wide samples at 100% and 200% text. Controls grow,
labels wrap and the example amount remains complete. A full-width destination
list replaces a cramped four-item bar without renaming or removing destinations.
The dark focused-action specimen uses the dark semantic roles.

Implementation must still test smaller/larger viewports, maximum supported
amounts, localized currency/date formats, native text settings, screen readers,
focus traversal, dialogs and reduced motion. Never silently shrink or ellipsize
a critical amount, sign, currency, date or error. Static specimens do not prove
all of those combinations.

The references use [WCAG 2.2 text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
and [non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)
thresholds of 4.5:1 and 3:1, respectively. The
[text-resize guidance](https://www.w3.org/WAI/WCAG22/Understanding/resize-text.html)
informs the 200% design probe; the
[target-size guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
does not replace platform-specific touch/accessibility requirements.

## Observed verification

| Board | Text nodes | Icons | Essential boundary samples | Clipping / contrast failures |
| --- | ---: | ---: | ---: | --- |
| Tokens | 40 | 4 | 0 | None |
| Components | 101 | 19 | 15 | None |
| States | 79 | 25 | 1 | None |
| Adaptive | 42 | 7 | 5 | None |

- All **262 actual text nodes**, **55 icons** and **21 essential boundary
  samples** passed their applicable thresholds. Minimum text/icon contrast was
  **5.39:1**; minimum sampled control-boundary contrast was **6.85:1**.
- Independent arithmetic checks on the JSON snapshot passed **44 semantic color
  pairs**, including light/dark text, links, action variants, disabled roles,
  boundaries, focus and feedback. Threshold checks precede rounding.
- Resolved Pen layouts showed no clipping, including the 320-wide large-text
  samples. Screenshots were visually inspected. `pdfinfo` confirmed four pages.
- The token snapshot is exported from Pen, not maintained as an independent
  manually chosen palette. Repository checks and native-save status are recorded
  in the [execution plan](../plans/active/2026-10-25-ui-design-system.md).
- No runtime behavior changed; functional RED/GREEN and native UI tests do not
  apply to these static artifacts. Native implementation checks remain explicit.

The PDF embeds font subsets. Existing [Bodoni Moda](visual-identity/licenses/bodoni-moda-OFL.txt)
and [IBM Plex Mono](visual-identity/licenses/ibm-plex-mono-OFL.txt) notices apply.

## Completion boundary

P01.10 is complete as an initial token, component and interface-state definition.
The native file was updated after the final canvas edits and exports. No commit,
PR, merge or publication is included. Phase 01 design work does not mean
financial features or mobile clients are implemented.
