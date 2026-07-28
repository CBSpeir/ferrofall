#!/usr/bin/env python3
"""Enforce Oxidefall's compressed critical-path budget."""

from __future__ import annotations

import gzip
import sys
from pathlib import Path


LIMIT_BYTES = 3 * 1024 * 1024
MUSIC_LIMIT_BYTES = 4 * 1024 * 1024
MUSIC_STEMS = {"music_base.ogg", "music_drive.ogg", "music_pressure.ogg"}


def main() -> int:
    dist = Path(sys.argv[1] if len(sys.argv) > 1 else "dist")
    critical = [dist / "index.html", *dist.glob("*.js"), *dist.glob("*_bg.wasm")]
    missing = [path for path in critical if not path.is_file()]
    if missing or not critical:
        names = ", ".join(str(path) for path in missing) or str(dist)
        print(f"critical web bundle is incomplete: {names}", file=sys.stderr)
        return 1

    compressed = sum(
        len(gzip.compress(path.read_bytes(), compresslevel=9, mtime=0))
        for path in critical
    )
    print(
        f"critical web path: {compressed / (1024 * 1024):.2f} MiB "
        f"compressed (limit {LIMIT_BYTES / (1024 * 1024):.0f} MiB)"
    )
    if compressed > LIMIT_BYTES:
        print("compressed critical path exceeds the mobile budget", file=sys.stderr)
        return 1

    audio_dir = dist / "audio"
    music = {path.name: path for path in audio_dir.glob("music_*.ogg")}
    if set(music) != MUSIC_STEMS:
        print("production bundle has missing or unexpected music stems", file=sys.stderr)
        return 1
    music_bytes = sum(path.stat().st_size for path in music.values())
    print(
        f"music assets: {music_bytes / (1024 * 1024):.2f} MiB "
        f"(limit {MUSIC_LIMIT_BYTES / (1024 * 1024):.0f} MiB)"
    )
    if music_bytes > MUSIC_LIMIT_BYTES:
        print("music assets exceed the release budget", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
