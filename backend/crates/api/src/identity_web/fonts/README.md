# Bundled identity fonts

Unmodified upstream TrueType files from Google Fonts. Both families retain their
upstream SIL Open Font License notices alongside the font binaries. No CDN or
download is used at runtime. Font updates are manual, never part of the build.

| Local file | Pinned source | SHA-256 |
| --- | --- | --- |
| `bodoni-moda.ttf` | [Source](https://raw.githubusercontent.com/google/fonts/6ce172f74aa355ea43eb964fa4a91570a4d3064d/ofl/bodonimoda/BodoniModa%5Bopsz,wght%5D.ttf) | `550f5e34ee0a828d7941b1fe9bc58b34e5260d3f33a61532e6d0a0114e79a5cf` |
| `BodoniModa-OFL.txt` | [Source](https://raw.githubusercontent.com/google/fonts/6ce172f74aa355ea43eb964fa4a91570a4d3064d/ofl/bodonimoda/OFL.txt) | `931dfe2e0cd3c9443295f5c1d754b8a8403bf060df50c1d5900267ade83d0046` |
| `ibm-plex-mono-regular.ttf` | [Source](https://raw.githubusercontent.com/google/fonts/01d5d9f05246fb7090a8403b81257535cb1a088b/ofl/ibmplexmono/IBMPlexMono-Regular.ttf) | `6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46` |
| `ibm-plex-mono-medium.ttf` | [Source](https://raw.githubusercontent.com/google/fonts/01d5d9f05246fb7090a8403b81257535cb1a088b/ofl/ibmplexmono/IBMPlexMono-Medium.ttf) | `a9b4c49bb299e05b5f6c481e7fb5e78943d2793249a0c8874ab574a2d1ea6755` |
| `IBMPlexMono-OFL.txt` | [Source](https://raw.githubusercontent.com/google/fonts/01d5d9f05246fb7090a8403b81257535cb1a088b/ofl/ibmplexmono/OFL.txt) | `7e6b2818edbd8f6a01ae80641cc8f16a51080d08fb4e532be3a0b6f74adb07da` |

Bodoni Moda supplies variable weight/optical-size axes. IBM Plex Mono supplies
regular (400) and medium (500); UI emphasis deliberately uses medium rather than
requesting an unbundled bold face. Preserve the font licenses when distributing
the API binary.
