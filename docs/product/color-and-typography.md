# Initial color palette and typography

**Roadmap:** P01.09 · **Status:** Initial exploration completed · **Date:** 2026-10-25

Use the supplied blue palette with one adjustment: **500 becomes `#2466D1`**.
Pair **Bodoni Moda** for large editorial headings with **IBM Plex Mono** for
financial figures and short UI copy. This is an initial direction, not a final
design system or an accessibility certification for the application.

## Review the boards

Open the [two-page PDF](visual-identity/export.pdf), or the individual boards:

| Board | Snapshot | Pen frame |
| --- | --- | --- |
| Palette, contrast table and safe pairings | [PNG](visual-identity/wzMWa.png) | `wzMWa` |
| Typography table and light/dark reading specimens | [PNG](visual-identity/XQvrg.png) | `XQvrg` |

The boards were added through the Pen MCP to the open
[design document](../cuenora-design.pen), to the right of the existing sketches.
The 27 P01.06–P01.08 root frames remain present; they were not restyled.
The user confirmed the native Pen save on 2026-10-25. The repository design file
has an updated timestamp and size, and Pen readback shows the latest reference
screens. Editable source and PNG/PDF review snapshots are retained locally.

### High-fidelity application references

Three additional [Home, Accounts and Transactions references](visual-identity/high-fidelity/README.md)
apply this palette and type pairing to a consistent synthetic ledger. See their
[three-page PDF](visual-identity/high-fidelity/export.pdf). They preserve the
original wireframes and remain static visual references, not implemented UI.
These screen additions are included in the user-confirmed native save.

## Palette

Ratios below compare each swatch with white (`#FFFFFF`) and ink 950 (`#040B16`).
Recommended swatch text passes the **4.5:1 normal-text threshold**. Ratios are
displayed to two decimals, but pass/fail decisions use unrounded values.

| Shade | Hex | White contrast | 950 contrast | Text on swatch | Initial use |
| --- | --- | ---: | ---: | --- | --- |
| 50 | `#E9F0FB` | 1.15 | 17.21 | 950 | Page tint |
| 100 | `#C2D5F4` | 1.49 | 13.26 | 950 | Subtle surface |
| 200 | `#9BBBED` | 1.96 | 10.07 | 950 | Selected surface, with a non-color selection cue |
| 300 | `#74A0E7` | 2.65 | 7.44 | 950 | Accent on dark backgrounds |
| 400 | `#4D85E0` | 3.65 | 5.40 | 950 | Underlined link on 900/950 |
| 500 | `#2466D1` | 5.39 | 3.66 | White | Primary action on white/50 |
| 600 | `#1F57B2` | 6.85 | 2.88 | White | Underlined link on white/50/100 |
| 700 | `#18448B` | 9.38 | 2.10 | White | Secondary text on white/50/100/200 |
| 800 | `#143770` | 11.60 | 1.70 | White | Strong text on light surfaces |
| 900 | `#0B1E3D` | 16.57 | 1.19 | White | Headings; dark background |
| 950 | `#040B16` | 19.73 | 1.00 | White | Primary ink; darkest background |

### Why change only 500?

The original `#266BD9` passes on white (5.02:1), but fails as normal text on
50 `#E9F0FB` (4.38:1). The slightly darker `#2466D1` passes both: **5.39:1 on
white and 4.70:1 on 50**. All ten other supplied values remain unchanged.
Light shades are useful surfaces, not automatically valid text colors on white.

| Foreground / background | Contrast | Use |
| --- | ---: | --- |
| 900 / 50, or 50 / 900 | 14.46:1 | Light/dark reading specimens |
| White / revised 500 | 5.39:1 | Primary action label |
| 600 / 100 | 4.61:1 | Link on tinted surface |
| 700 / 200 | 4.79:1 | Text on selected surface |
| 400 / 900 | 4.53:1 | Dark-mode link; modest contrast margin |

Prefer 300 rather than 400 for dark-mode links when a greater contrast margin is
needed. Do not apply reduced opacity to these pairings without recalculating.
The selected-state tint and decorative table dividers are not sufficient
boundaries for interactive controls.

## Typography

| Role | Family | Weight | Initial size / line height |
| --- | --- | --- | --- |
| Display | Bodoni Moda | 500 | 48–64 px / 1.15 |
| Short heading | Bodoni Moda | 500–600 | 32 px / 1.25 |
| Short UI copy | IBM Plex Mono | 400 | 16 px / 1.5 |
| Financial amount | IBM Plex Mono | 500 | 20–32 px / 1.3 |
| Control label | IBM Plex Mono | 500 | 16 px / 1.5 |

These are specimen sizes, not fixed native accessibility limits. Mobile
implementation must map them to scalable platform typography. Smaller board
annotations describe the design and are not proposed product text sizes.

**Bodoni Moda** supplies the high stroke contrast and fine serifs sought in a
Didot-like style; this is a design judgment, not a claim that it reproduces
Linotype Didot. Its variable distribution includes weight and optical-size axes.
Keep it out of small labels, dense tables and critical amounts. Optical-size
behavior must be verified in the eventual native renderer; the Pen schema does
not expose an optical-size control.

**IBM Plex Mono** gives figures equal-width characters and a restrained technical
counterpart to the display serif. Use explicit currency labels, right-aligned
amounts, and separate totals for SEK, EUR and USD. Monospacing costs horizontal
space: allow wrapping, avoid long prose, and never shrink or truncate amounts
to fit. If full-screen trials reveal cramped copy, a proportional sans is the
follow-up to evaluate, not a reason to reduce font size.

The boards include `0/O`, `1/I/l`, `5/S`, `8/B`, accented letters, currency symbols,
and a true minus sign. The sample balances are synthetic and do not prescribe a
locale-specific formatting policy.

### Licensing and sources

- [Linotype Didot](https://www.myfonts.com/collections/didot-font-linotype) is
  commercially licensed. It is not selected or bundled.
- [Bodoni Moda license](https://raw.githubusercontent.com/google/fonts/main/ofl/bodonimoda/OFL.txt):
  SIL Open Font License 1.1. See the [font metadata](https://raw.githubusercontent.com/google/fonts/main/ofl/bodonimoda/METADATA.pb)
  for supported axes and the [designer's project](https://indestructibletype.com/Bodoni.html)
  for the design background.
- [IBM Plex Mono license](https://raw.githubusercontent.com/google/fonts/main/ofl/ibmplexmono/OFL.txt):
  SIL Open Font License 1.1; see the [IBM Plex project](https://github.com/IBM/plex).

OFL permits use, embedding and redistribution subject to its conditions. Preserve
each font's copyright and license notices when bundling; font files are not
relicensed under Cuenora's MIT license. No standalone font binaries are added in
P01.09. The PDF embeds subsets; the [Bodoni Moda notice](visual-identity/licenses/bodoni-moda-OFL.txt)
and [IBM Plex Mono notice](visual-identity/licenses/ibm-plex-mono-OFL.txt) accompany it.
Sources were checked on 2026-10-25.

## Accessibility rules and verification

- [WCAG 2.2 text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html):
  normal text needs 4.5:1; large text needs 3:1. These boards use at least 4.5:1
  for all actual text, including large headings.
- [Non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html):
  essential icons, control boundaries and state indicators need 3:1 against
  adjacent colors. Choose focus styling for each actual background in P01.10.
- Underline links and pair states with labels/icons. Never distinguish income,
  expenses, errors or chart series by color alone. This blue ramp is not a
  categorical chart palette or a complete semantic status palette.
- Support larger text, wrapping and native accessibility settings. Screen-reader
  semantics, native font rendering, text scaling and complete component states
  cannot be proven by static design boards; they remain implementation checks.

Observed checks:

- All 11 shades and the documented foreground/background ratios were calculated
  using sRGB relative luminance; pass/fail comparisons precede rounding.
- Pen readback found no clipped nodes on either completed board.
- Actual text/background audit: **96 palette text nodes**, minimum **5.39:1**;
  **52 typography text nodes**, minimum **9.38:1**; zero normal-text contrast failures.
- Both PNGs were visually inspected and the PDF exported for review.
- The documented contrast check, `python3 scripts/check_repository.py`, and
  `git diff --check` passed. `pdfinfo` confirmed two pages; `pdffonts` confirmed
  embedded Bodoni Moda 500 and IBM Plex Mono regular/medium subsets.
- No runtime behavior changed, so a functional RED/GREEN cycle is not applicable.

To reproduce the contrast matrix without dependencies:

```bash
python3 - <<'PY'
colors = ['E9F0FB', 'C2D5F4', '9BBBED', '74A0E7', '4D85E0',
          '2466D1', '1F57B2', '18448B', '143770', '0B1E3D', '040B16']
def luminance(value):
    rgb = [int(value[i:i+2], 16) / 255 for i in (0, 2, 4)]
    linear = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055)**2.4
              for v in rgb]
    return sum(v * w for v, w in zip(linear, (0.2126, 0.7152, 0.0722)))
def contrast(a, b):
    low, high = sorted((luminance(a), luminance(b)))
    return (high + 0.05) / (low + 0.05)
for index, color in enumerate(colors):
    foreground = '040B16' if index < 5 else 'FFFFFF'
    assert contrast(color, foreground) >= 4.5
    print(color, f'white={contrast(color, "FFFFFF"):.2f}',
          f'950={contrast(color, "040B16"):.2f}')
assert contrast('266BD9', 'E9F0FB') < 4.5
assert contrast('2466D1', 'E9F0FB') >= 4.5
assert contrast('1F57B2', 'C2D5F4') >= 4.5
assert contrast('18448B', '9BBBED') >= 4.5
assert contrast('4D85E0', '0B1E3D') >= 4.5
PY
```

## Next step

P01.09 is complete as an initial palette and typography exploration. The
[P01.10 design system](ui-design-system.md) defines the initial semantic tokens,
component states and adaptive typography specimens. Native implementation and
device testing remain separate. Existing grayscale wireframes stay historical
low-fidelity references. Commit and pull-request integration are separate delivery steps.
