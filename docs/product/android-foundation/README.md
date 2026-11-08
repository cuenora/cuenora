# Android native foundation — P11.01

The Android host implements Cuenora's theme, navigation and reusable Compose
primitives. It is not a signed-in financial client: destination bodies explicitly
state that data is not connected, and component examples are synthetic.

## Design references

[Design guide](../../../DESIGN.md), [UI contract](../ui-design-system.md) and
[low-fidelity navigation](../navigation-and-flows.md) remain authoritative.
Editable additions live in [cuenora-design.pen](../../cuenora-design.pen):

| Pen frame | Export | Purpose |
| --- | --- | --- |
| `U4kA6` | [Light](U4kA6.png) | Android bottom navigation and brand hierarchy |
| `waFiO` | [Dark](waFiO.png) | Dark role pairing, not inverted colors |
| `WhUwV` | [Destination list](WhUwV.png) | Full-width targets for narrow/large-text layouts |
| `D3u3t` | [Components](D3u3t.png) | Base specimens, not persisted financial records |

Existing wireframes and platform-neutral component sources are unchanged. These
new boards describe Android adaptation, not a replacement iOS design. Boards show
full content; native system insets are implemented by Compose, not copied artwork.

Actual API37 emulator captures: [light](runtime-light.png) and
[dark](runtime-dark.png). These are the foundation shell, not populated financial
screens. The emulator was restored to its original light-mode setting.

## Platform decisions

- Material3 supplies native control semantics, ripple, dialog and navigation
  behavior. There is no floating translucent iOS-style capsule bar.
- Four existing top-level destinations remain Home, Accounts, Transactions and
  Insights. Navigation Compose owns the selected route, restoration and Back.
  Reselecting a destination does not duplicate it; switching roots returns through
  Home instead of building an arbitrary history of tab taps.
- Below 440dp, or above 1.2 font scale, the app exposes the same destinations in a
  scrollable native drawer. Drawer targets grow with font scale. From 440dp it
  uses a bottom bar; from 600dp it uses a navigation rail. Breakpoints deliberately
  reserve room for the full monospaced labels instead of shrinking text.
- Native Back closes the drawer before navigating. The component catalog uses an
  Up arrow. Navigation transitions are disabled, so optional motion is not forced.
- System light/dark mode selects the documented semantic colors; wallpaper-derived
  dynamic colors do not override the brand. Fonts use scalable `sp`, geometry `dp`.
- Native controls and focus indicators remain visible; a 2dp focus ring has an
  allocated 2dp gap. Buttons have a 56dp baseline and grow with content. Disabled
  actions expose a reason; busy actions prevent repeated activation. Safe dialog
  focus starts on Keep editing, never Discard.

## Reuse in later screens

`CuenoraTheme`, `CuenoraAction`, `CuenoraField`, `CuenoraNotice`, `CuenoraAmount`,
`CuenoraStatus`, `CuenoraLoading` and `CuenoraDiscardDialog` live in the Android
`ui` package. Material3 supplies ordinary checkbox/radio/switch/secondary controls
under the same theme; no competing generic component framework is introduced.
Amount display accepts an explicitly formatted exact string and currency, never
parses a floating-point amount. Later presentation logic must provide correct
localized values and preserve each currency separately.

The Home link opens a clearly labeled component preview with no repository,
network request or write. P11.02 onward supplies real authentication, destination
content and financial workflows. This foundation does not bypass server gates or
claim offline writes, session recovery, or working financial screens.

## Font provenance

Fonts were obtained from the public [Google Fonts repository](https://github.com/google/fonts)
on 2026-11-08: `ofl/bodonimoda/BodoniModa[opsz,wght].ttf`,
`ofl/ibmplexmono/IBMPlexMono-Regular.ttf` and `IBMPlexMono-Medium.ttf`.
Bodoni Moda was statically instantiated at weight 500 and optical size 32 using
fontTools, so API 23 does not need variable-font support. IBM files are unchanged.
Both SIL OFL notices are packaged under `src/main/assets/licenses/` in the APK;
the original notices also remain in the visual-identity directory.

## Verification limits

See the [execution record](../../plans/active/2026-11-08-android-native-foundation.md)
for observed results. Automated semantics/layout tests and static contrast checks
are not TalkBack, Switch Access, hardware keyboard, every localization or every
supported Android-version certification. P11.11/P11.12 retain broader device and
recovery coverage; foundation regressions are tested now, not deferred to them.

### Completed foundation checks (2026-11-08)

- [x] Four new Pen boards saved and exported; existing reference layers retained.
- [x] Native Home visually inspected in system light/dark modes.
- [x] Thirteen instrumented tests passed on API37, with no failures/errors/skips.
- [x] Independent app assemble/lint and 76 shared host tests passed separately.
- [x] Actual text/selection/notice contrast pairs exceed 4.5:1 (minimum 6.53:1
  among the checked pairs); arbitrary future combinations remain unverified.

Lint has 0 errors and 4 unsuppressed warnings: old target, extraction rules,
missing launcher icon, and Navigation newer-version notice. Navigation2.10
requires API24; this task preserves the existing API23 minimum. A combined
lint/shared-test invocation fails a fixture-generation dependency check on both
baseline and candidate; use the separately documented commands. Wider TalkBack,
physical-device, API23-runtime and financial-flow checks remain unperformed.
