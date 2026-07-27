# Ferrofall sound effects

These 22 mono, 16-bit, 32-kHz WAV files are original procedural sound effects
for Ferrofall. They are generated deterministically from the Python standard
library:

```sh
python3 tools/generate_audio.py
```

Verify that the checked-in bank matches the generator with:

```sh
python3 tools/generate_audio.py --check
```

The complete generated bank is dedicated to the public domain under CC0 1.0.
See [LICENSE-CC0.txt](LICENSE-CC0.txt). The generator itself is source code and
uses Ferrofall's MIT-or-Apache-2.0 license.
