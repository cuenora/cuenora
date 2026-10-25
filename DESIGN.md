# Cuenora design guide

Cuenora combines **editorial warmth with precise financial information**:
Bodoni Moda gives short headings character; IBM Plex Mono keeps figures and
short interface copy orderly. A restrained blue palette supports clarity rather
than making the product look or sound like a bank.

This is the entry point for the initial P01.09–P01.10 design direction. The
artifacts are reference designs, not implemented mobile screens, a working theme
system or a certification of application accessibility.

## Start here

| Reference | Purpose |
| --- | --- |
| [Editable Pen document](docs/cuenora-design.pen) | Boards, components and screen layers |
| [High-fidelity screens — PDF](docs/product/visual-identity/high-fidelity/export.pdf) | Home, Accounts and Transactions |
| [Design system — PDF](docs/product/design-system/export.pdf) | Tokens, components, states and adaptive examples |
| [UI design system](docs/product/ui-design-system.md) | Detailed semantic roles, component inventory and interaction rules |
| [Token snapshot](docs/product/design-system/tokens.json) | 76 named `p0110-*` design-reference variables |
| [Palette and typography](docs/product/color-and-typography.md) | Color exploration, contrast measurements and font rationale |

Use this guide for orientation and the detailed system guide for component
specifications. Pen holds editable definitions; JSON and image/PDF exports are
snapshots that must stay synchronized. The JSON is a design-reference format,
not a production configuration or a platform theme generator.

## Design principles

- **Make the financial meaning visible.** Recorded, planned, estimated and
  uncertain are different states, not interchangeable visual treatments.
- **Build trust through accuracy.** Explain what is known, what is unavailable
  and what the person can do next. Do not imply live bank data or payment execution.
- **Keep the interface calm.** Use clear hierarchy, open rows, restrained surfaces
  and whitespace rather than excessive cards, gradients, shadows or decoration.
- **Respect native behavior.** Share concepts and financial semantics while
  retaining platform-appropriate navigation, controls and accessibility behavior.
- **Let content grow.** Adapt layout before shrinking type or hiding a critical
  amount, currency, date, sign or error message.

Product language follows the [verbal identity](docs/product/verbal-identity.md):
clear, approachable, trustworthy and non-banking. Use sentence case and neutral
English. Describe the action without blame, pressure or moral judgments about
income and spending. Uppercase reference-board annotations are not a product-copy
convention.

## Color

Use semantic tokens for new components, not arbitrary shades from the ramp.
The initial blue palette is preserved in `p0109-*`; revised **500 is `#2466D1`**,
not the original `#266BD9`. The `p0110-light-*` and `p0110-dark-*` roles define
how those colors are used.

| Role | Light | Dark |
| --- | --- | --- |
| Canvas | `#FFFFFF` | `#040B16` |
| Surface | `#FFFFFF` | `#0B1E3D` |
| Muted surface | `#E9F0FB` | `#143770` |
| Primary text | `#0B1E3D` | `#E9F0FB` |
| Secondary text | `#18448B` | `#9BBBED` |
| Essential control boundary | `#1F57B2` | `#74A0E7` |
| Primary action / label | `#2466D1` / white | `#9BBBED` / `#040B16` |
| Underlined link | `#1F57B2` | `#9BBBED` |
| Focus indicator | `#18448B` | `#C2D5F4` |

Color accessibility belongs to a **foreground/background pair**, not a swatch.
Use the documented pairings and recalculate contrast when background, opacity
or adjacency changes. Decorative separators are not essential control boundaries.

Information uses blue; confirmed process success uses green; attention and
uncertain outcomes use amber; errors use a restrained red. Each includes a label
and an icon or another non-color cue. Feedback notices retain their own opaque
surfaces in either mode. These accents are not an income/expense classification
or a categorical chart palette. Exact values and verified pairings are in the
[semantic token contract](docs/product/ui-design-system.md#token-contract).

## Typography

| Role | Family | Initial design size |
| --- | --- | --- |
| Short display title | Bodoni Moda 500 | 44 |
| Short heading | Bodoni Moda 500 | 32 |
| Financial amount | IBM Plex Mono 500 | 24 |
| Short UI copy and control labels | IBM Plex Mono 400–500 | 16 |
| Metadata | IBM Plex Mono 400–500 | 14 |
| Compact navigation | IBM Plex Mono 400–500 | 13 |

Body line height is 1.5; heading line height is 1.2. These are design references,
not fixed native text sizes. Larger P01.09 headline samples are exploratory
specimens; use the P01.10 token baseline when building new components.

Keep fine display serifs out of small labels and critical amounts. Monospacing
helps align figures but consumes width: wrap copy, use vertical layout or switch
to the destination-list navigation pattern when necessary. Do not squeeze a
four-item bar until labels become illegible.

Both selected families use SIL OFL 1.1. Preserve the
[Bodoni Moda](docs/product/visual-identity/licenses/bodoni-moda-OFL.txt) and
[IBM Plex Mono](docs/product/visual-identity/licenses/ibm-plex-mono-OFL.txt)
notices when bundling them; Cuenora's MIT license does not replace font licenses.
Copied OS-status specimens also retain an
[Inter notice](docs/product/visual-identity/licenses/inter-OFL.txt).

## Layout and components

- Spacing scale: **4, 8, 12, 16, 20, 24, 32, 48** design units.
- Control/card radii: **10 / 12**. Use capsules intentionally, not on every surface.
- Minimum target: **48**; baseline control height: **56**. Grow with text rather
  than treating the baseline as a fixed height. Map dimensions to native units.
- Icon size: **24**. Default boundary: **1**. Focus: **2** with a **2-unit gap**.
- Give each screen a clear location and primary intent. Reserve cards for
  meaningful grouping, such as balance summaries, planning or a two-sided transfer.
- Reuse actions, labeled fields, selectors, selection controls, amount displays,
  account rows, notices, loading placeholders and safe confirmation dialogs.

The [component inventory](docs/product/ui-design-system.md#component-inventory)
lists all 17 reusable sources and their Pen IDs. The existing four destinations
are Home, Accounts, Transactions and Insights. Large-text navigation retains
those destinations in a full-width list rather than renaming or hiding them.

The high-fidelity phone frames show **full scroll content**, not fixed device
viewport heights. Native system chrome and navigation belong outside the
scrolling body. The 320-wide examples illustrate adaptation; they do not prove
every viewport or localization.

## States must preserve meaning

| Situation | Design rule |
| --- | --- |
| Loading or failed read | Show unavailable/busy data, never fabricated zero balances |
| No records vs. no filter matches | Explain the difference and offer the relevant create or filter action |
| Invalid input | Preserve values, identify the correction and apply no partial financial effect |
| Submission in progress | Explain progress and prevent a second submission |
| Confirmed write | Confirm the actual result and link to its existing record |
| Unknown write outcome | Claim neither success nor failure; resolve the original operation |
| Session expiry or denied access | Protect sensitive data; never silently resubmit uncertain work |
| Unsaved edits | Offer Keep editing and explicit discard; cancellation changes no records |

Focus, pressed, selected and disabled are distinct interaction states. Disabled
controls need a visible reason and native semantics, not just reduced opacity.
Focus remains visible; dialogs start with the safe action and return focus to
their invoker. Optional transitions use 120/180 ms and respect reduced motion.
These are implementation requirements, not behavior executed by static Pen layers.

Use the [state catalog](docs/product/ui-design-system.md#interface-state-contract)
and [shared recovery paths](docs/product/navigation-and-flows.md#shared-alternate-and-recovery-paths)
for detailed copy. Do not invent retry controls without the implemented
idempotency/status contract, or treat deletion of posted movements as routine cleanup.

## Financial presentation

- Keep SEK, EUR and USD totals separate. No implied converted grand total.
- Show currency, sign and full amount; align comparable financial values.
- Accountless income/expenses remain in reports but do not affect balances.
- Transfers are one operation with two account movements, not income or spending.
- Planned payments and estimated interest do not change recorded balances.
- A planning difference is not the account balance or guaranteed money available.
- Dates alone never confirm payments or interest receipts. Confirm only real events.
- Use synthetic data in reference designs, fixtures and public screenshots.

The [financial glossary](docs/product/glossary.md) and product requirements own
the business meaning. A mockup must not introduce a financial rule. If a screen,
token snapshot and specification disagree, investigate and synchronize them
rather than silently choosing whichever is easiest to implement.

## Accessibility and maintenance

Target at least **4.5:1 for normal text** and **3:1 for essential non-text cues**
using actual adjacent colors. Do not rely on color alone. The detailed guide
records static checks, their sources and limits; those checks do not replace
native screen-reader, keyboard/focus, text-scaling or device verification.

When changing the design:

1. Reuse an existing semantic role/component before introducing another one.
2. Update the editable Pen definitions and the affected detailed specification.
3. Re-export the token snapshot and relevant PNG/PDF references; check they match.
4. Recheck contrast, boundaries, clipping, narrow/large-text layouts and financial meaning.
5. Save the native `.pen` document and retain the applicable font notices.

Before implementing a client, verify maximum supported amounts, localized
formats, native text settings, assistive technology, focus traversal, dialogs
and reduced motion. This guide does not authorize an implementation or imply
that the designed screens already exist in the product.
