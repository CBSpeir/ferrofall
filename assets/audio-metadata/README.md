# Oxidefall audio assets

All runtime audio in `../audio` is original, procedurally generated material
for Oxidefall. This directory contains its provenance, hashes, and budgets so
metadata is not copied into the web bundle.

## Sound effects

The 22 mono, 32-kHz Ogg Vorbis effects are rendered deterministically from
lossless WAV masters generated with the Python standard library:

```sh
python3 tools/generate_audio.py
python3 tools/generate_audio.py --check
```

Lossless masters are written under `target/audio-masters` and are not shipped.
The checked-in effects must remain below 256 KiB combined.

## Adaptive music

The three mono, 32-kHz Ogg Vorbis stems form a synchronized 64-bar loop at
132 BPM:

- `music_base.ogg` supplies the permanent foundation;
- `music_drive.ogg` enters at level 5 or when danger advances a base-tier run;
  and
- `music_pressure.ogg` enters at level 10 or when danger advances a drive-tier
  run.

The clean-room composition is modal, industrial-electronic, and generated
without importing or transcribing third-party music. Its source generator,
musical event data, asset hashes, peak levels, duration, and byte budget are
recorded by:

```sh
python3 tools/generate_music.py
python3 tools/generate_music.py --check
```

FFmpeg is the preferred Vorbis encoder. When it is unavailable, the generator
uses the source-built Rust encoder in `tools/music_encoder`. Lossless WAV
masters are written under `target/music-masters` and are not shipped.

## License

The generated effects, rendered music stems, and composition data are
dedicated to the public domain under CC0 1.0. See
[LICENSE-CC0.txt](LICENSE-CC0.txt). Generator and encoder source code use
Oxidefall's MIT-or-Apache-2.0 license.
