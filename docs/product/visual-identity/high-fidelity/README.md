# High-fidelity mobile references

Three static references apply the [P01.09 palette and typography](../../color-and-typography.md)
to Home, Accounts and Transactions. They are visual direction, not implemented
screens, an interactive prototype or a complete P01.10 component system.

## Review

Open the [three-page PDF](export.pdf), or inspect a screen:

| Screen | Snapshot | Pen frame | Full-content size |
| --- | --- | --- | --- |
| Home | [PNG](e0RcrM.png) | `e0RcrM` | 440 × 1812 |
| Accounts | [PNG](P4ArR.png) | `P4ArR` | 440 × 1623 |
| Transactions | [PNG](x0eotx.png) | `x0eotx` | 440 × 1878 |

Editable layers are in the [Pen document](../../../cuenora-design.pen), below the
palette board, under `P01.09 · High fidelity`. The user confirmed the native save
on 2026-10-25; the repository file has been updated and the latest screens remain
present in Pen readback. PNG/PDF snapshots are also saved. The original 27
low-fidelity frames and both palette/typography boards were preserved.

## Design direction

- Bodoni Moda 500 gives short titles an editorial hierarchy. IBM Plex Mono
  carries amounts, labels and short copy. The copied OS status bar keeps its
  original typography.
- White and 50 are the main surfaces; 900 anchors balance/period summaries.
  Revised 500 (`#2466D1`) identifies primary actions; 600 identifies underlined
  links. Text and icons do not rely on low-contrast light-blue shades on white.
- Two reusable reference components keep primary actions and selected navigation
  consistent: `Vuzih` and `t59Pc`. Selection has an outline and font-weight cue,
  not just a background color. These are not a complete component library.
- Cards are reserved for balances, planning and the two-sided transfer. Ordinary
  account/activity entries remain open rows with separators.

The frames show full scroll content, not device viewport heights. Navigation is
shown after the full content for inspection; implementation should keep system
chrome and navigation outside the scrolling body. A 440 px reference does not
prove smaller-width layouts or dynamic text behavior.

## Consistent financial example

All three references use the existing [P01.07 synthetic ledger](../../accounts-and-transactions-wireframes.md#synthetic-ledger-and-walkthrough):

- Recorded balances: A SEK 2,500; B SEK 500; Savings S SEK 1,000; Euro EUR 118;
  Dollar USD 100. Totals stay separate: SEK 4,000 / EUR 118 / USD 100.
- A's SEK 800 planned payment gives a SEK 1,700 planning difference. It does
  not reduce the recorded SEK 2,500 balance or become realized spending.
- The accountless SEK 200 transport expense counts as spending but changes no
  account balance. The SEK 200 / EUR 18 transfer is one operation, not spending
  or income. The expense and income signs and type labels remain explicit.

Home therefore uses the same ledger as Accounts/Transactions, **not** the
independent EX-08 fixture in the original grayscale Home. The October period
and planning date are illustrative, not new product defaults. No real financial
data or financial operation is involved.

## Observed checks

- Pen resolved-layout audit: zero clipping problems on all three screens.
- Actual foreground/background contrast: **111 text nodes and 56 icons** checked;
  minimum **5.39:1** for each category, with zero failures against 4.5:1 text and
  3:1 icon thresholds. Ratios were compared before rounding.
- Navigation targets are 100 × 64; primary actions are 56 high; links and date
  targets are 48 high in design coordinates. Native platform sizes, accessible
  names, focus order and assistive technologies still need implementation checks.
- Screen captures were visually inspected; decimal arithmetic checks confirmed
  balance totals, transfer effects, accountless spending and the planning
  difference. `pdfinfo` confirmed three pages.
- Body copy starts at 16; compact metadata/navigation use 12–14. These are
  reference choices, not an excuse to prevent text scaling or truncate amounts.
- No runtime code changed; no runtime test suite or RED/GREEN cycle applies.

The PDF embeds font subsets; the [Bodoni Moda](../licenses/bodoni-moda-OFL.txt),
[IBM Plex Mono](../licenses/ibm-plex-mono-OFL.txt) and copied OS-status
[Inter](../licenses/inter-OFL.txt) notices apply. The Inter notice comes from
the [official project](https://github.com/rsms/inter/blob/master/LICENSE.txt).

The [P01.10 design system](../../ui-design-system.md) now defines initial tokens,
component states and narrow/large-text specimens. Real viewport and assistive
technology validation remain native implementation work. These references
complete the requested P01.09 visual exploration, not native UI implementation
or a full accessibility certification.
