# Cuenora logo collection

Review the [five-page brand reference](export.pdf). The collection follows the
existing [design guide](../../../../DESIGN.md), blue palette, Bodoni Moda wordmark
and IBM Plex Mono supporting typography. It is a design proposal, not a claim of
trademark clearance or a change to implemented application UI.

## Choose an asset

The `png/` directory contains 16 transparent logo exports: four color treatments
times four compositions. Files use `cuenora-{treatment}-{composition}.png`.

| Treatment | Symbol | Wordmark | Intended background |
| --- | --- | --- | --- |
| `light` | `#2466D1` | `#0B1E3D` | White or the existing light canvas |
| `dark` | `#9BBBED` | `#E9F0FB` | `#040B16` |
| `black` | `#000000` | `#000000` | White; single-ink reproduction |
| `white` | `#FFFFFF` | `#FFFFFF` | Dark; reversed single-color reproduction |

Compositions are `symbol`, `horizontal` (symbol and name), `stacked` (symbol
above name), and `tagline` (symbol, name and approved message). The message is
“Open-source personal finance, clearly organized.” Its line break is visual only.

The four `svg/cuenora-{treatment}-symbol.svg` files retain the generated vector
geometry and need no fonts. The PNG wordmarks and PDF reference provide rendered
compositions; editable typography uses the existing project font families and
their [retained notices](../licenses/). The additional
`png/cuenora-app-tile-blue.png` is an opaque blue tile with a white symbol, not a
complete platform-specific app-icon package.

## Editable source and previews

The open [Pen document](../../../cuenora-design.pen) contains a reusable
`Brand / Master / Cuenora symbol` component (`peax8`), four collection boards and
a small-size usage board. Existing P01.06–P01.10 screens and tokens are unchanged.

| Board | Preview | Pen frame |
| --- | --- | --- |
| Light | [PNG](boards/cuenora-light.png) | `Qd3y8` |
| Dark | [PNG](boards/cuenora-dark.png) | `riNhn` |
| Black | [PNG](boards/cuenora-black.png) | `K8eQf4` |
| White | [PNG](boards/cuenora-white.png) | `jfk4z` |
| Small sizes and app tile | [PNG](boards/cuenora-usage.png) | `Q4wqaw` |

The native document was subsequently saved in Pen. Its on-disk timestamp advanced
to 2026-10-29 22:02:49 +01:00 after the logo and email additions; the editable
source and persisted exports are included together for review.

## Reproduction

- Keep clear space of at least one-quarter of the symbol frame width.
- Preserve the silhouette and aspect ratio; do not rotate or recolor its bands
  independently. Backgrounds on reference boards are not part of the artwork.
- Prefer the symbol alone when the name or tagline becomes difficult to read.
  The usage board illustrates 24, 32, 48 and 96-pixel symbol frames; these are
  visual specimens, not certification of every device or print process.
- Use the approved light/dark pairings rather than arbitrary backgrounds.

## Verification

All five boards were visually inspected in Pen. A subsequent resolved-bounds
read found no clipping. The 16 PNG logos were checked for RGBA transparency,
including transparent corners; the four SVG symbols use paths without fonts or
background rectangles. The native document and exports are persisted. No
application behavior or roadmap completion changed.
