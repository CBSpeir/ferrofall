#!/usr/bin/env python3
"""Enforce Oxidefall's compressed critical-path budget."""

from __future__ import annotations

import gzip
import json
import sys
from pathlib import Path


CRITICAL_LIMIT_BYTES = int(1.5 * 1024 * 1024)
RUNTIME_LIMIT_BYTES = int(3.5 * 1024 * 1024)
ROOT = Path(__file__).resolve().parents[1]


def expected_audio_assets() -> set[str]:
    effects = json.loads(
        (ROOT / "assets" / "audio-metadata" / "effects_manifest.json").read_text()
    )
    music = json.loads(
        (ROOT / "assets" / "audio-metadata" / "music_manifest.json").read_text()
    )
    return {
        *(effect["asset"] for effect in effects["effects"].values()),
        *(stem["asset"] for stem in music["stems"].values()),
    }


def main() -> int:
    dist = Path(sys.argv[1] if len(sys.argv) > 1 else "dist")
    scripts = sorted(dist.glob("*.js"))
    wasm = sorted(dist.glob("*_bg.wasm"))
    favicons = sorted(dist.glob("favicon*.svg"))
    critical = [dist / "index.html", *favicons, *scripts, *wasm]
    missing = [path for path in critical if not path.is_file()]
    if missing or not scripts or not wasm or not favicons:
        names = ", ".join(str(path) for path in missing)
        if not scripts:
            names = f"{names}, JavaScript".lstrip(", ")
        if not wasm:
            names = f"{names}, WebAssembly".lstrip(", ")
        if not favicons:
            names = f"{names}, favicon".lstrip(", ")
        print(f"critical web bundle is incomplete: {names}", file=sys.stderr)
        return 1

    critical_bytes = sum(
        len(gzip.compress(path.read_bytes(), compresslevel=9, mtime=0))
        for path in critical
    )
    print(
        f"critical web path: {critical_bytes / (1024 * 1024):.2f} MiB "
        f"compressed (limit {CRITICAL_LIMIT_BYTES / (1024 * 1024):.1f} MiB)"
    )
    if critical_bytes > CRITICAL_LIMIT_BYTES:
        print("compressed critical path exceeds the mobile budget", file=sys.stderr)
        return 1

    audio_dir = dist / "audio"
    audio = {path.name: path for path in audio_dir.glob("*") if path.is_file()}
    expected_audio = expected_audio_assets()
    if set(audio) != expected_audio:
        missing_audio = sorted(expected_audio - set(audio))
        unexpected_audio = sorted(set(audio) - expected_audio)
        print(
            "production audio mismatch: "
            f"missing={missing_audio}, unexpected={unexpected_audio}",
            file=sys.stderr,
        )
        return 1
    audio_bytes = sum(path.stat().st_size for path in audio.values())
    runtime_bytes = critical_bytes + audio_bytes
    print(
        f"complete runtime path: {runtime_bytes / (1024 * 1024):.2f} MiB "
        f"transferred (limit {RUNTIME_LIMIT_BYTES / (1024 * 1024):.1f} MiB)"
    )
    if runtime_bytes > RUNTIME_LIMIT_BYTES:
        print("complete runtime path exceeds the transfer budget", file=sys.stderr)
        return 1
    if (dist / ".stage").exists():
        print("production bundle contains Trunk staging artifacts", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
