# Bundled fonts

Oxidefall's source fonts come from the Google Fonts repository:

- Saira Condensed ExtraBold for the wordmark and major headings;
- IBM Plex Mono Medium for controls, labels, statistics, and numbers.

The unmodified upstream files live in `source/`. Production bundles use the
UI-specific `OxidefallDisplay.ttf` and `OxidefallMono.ttf` subsets. The
subsets remove reserved upstream font names, as required for modified fonts
under the SIL Open Font License 1.1. The complete license texts are included
as `OFL-SairaCondensed.txt` and `OFL-IBMPlexMono.txt` in this directory.

Regenerate the subsets with HarfBuzz `hb-subset`, or verify their hashes and
UI glyph coverage without HarfBuzz:

```sh
python3 tools/generate_fonts.py
python3 tools/generate_fonts.py --check
```

- [Saira Condensed source][saira]
- [IBM Plex Mono source][plex]

[saira]: https://github.com/google/fonts/tree/main/ofl/sairacondensed
[plex]: https://github.com/google/fonts/tree/main/ofl/ibmplexmono
