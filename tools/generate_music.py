#!/usr/bin/env python3
"""Generate Oxidefall's deterministic, original adaptive music stems."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import struct
import subprocess
import sys
import wave
from array import array
from pathlib import Path


SAMPLE_RATE = 32_000
BPM = 132
BEAT_SAMPLES = round(SAMPLE_RATE * 60 / BPM)
BAR_SAMPLES = BEAT_SAMPLES * 4
BARS = 64
TOTAL_SAMPLES = BAR_SAMPLES * BARS
MUSIC_BUDGET_BYTES = 2 * 1024 * 1024
TAU = math.tau
ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "assets" / "audio"
MASTER_DIR = ROOT / "target" / "music-masters"
MANIFEST_PATH = ROOT / "assets" / "audio-metadata" / "music_manifest.json"
STEMS = ("music_base", "music_drive", "music_pressure")


def sample_at(bar: int, beat: float) -> int:
    return bar * BAR_SAMPLES + round(beat * BEAT_SAMPLES)


def midi_frequency(note: int) -> float:
    return 440.0 * 2.0 ** ((note - 69) / 12.0)


def envelope(index: int, length: int, attack: int, release: int) -> float:
    attack_gain = min(1.0, index / max(1, attack))
    release_gain = min(1.0, (length - index - 1) / max(1, release))
    return max(0.0, min(attack_gain, release_gain))


def add_tone(
    buffer: array[float],
    start: int,
    length: int,
    note: int,
    amplitude: float,
    voice: str,
    attack: float = 0.01,
    release: float = 0.08,
) -> None:
    end = min(start + length, TOTAL_SAMPLES)
    if start < 0 or start >= end:
        return
    frequency = midi_frequency(note)
    attack_samples = round(attack * SAMPLE_RATE)
    release_samples = round(release * SAMPLE_RATE)
    phase_step = TAU * frequency / SAMPLE_RATE
    for absolute in range(start, end):
        local = absolute - start
        phase = phase_step * local
        gain = envelope(local, end - start, attack_samples, release_samples)
        if voice == "sub":
            value = math.sin(phase) + 0.18 * math.sin(phase * 2.0)
        elif voice == "pulse":
            value = (
                math.sin(phase)
                + 0.34 * math.sin(phase * 3.0)
                + 0.12 * math.sin(phase * 5.0)
            ) / 1.34
        elif voice == "glass":
            value = (
                math.sin(phase)
                + 0.28 * math.sin(phase * 2.03 + 0.3)
                + 0.17 * math.sin(phase * 3.91 + 1.1)
            ) / 1.35
        else:
            value = (
                math.sin(phase)
                + 0.22 * math.sin(phase * 1.5 + 0.8)
                + 0.12 * math.sin(phase * 2.5 + 0.2)
            ) / 1.25
        buffer[absolute] += amplitude * gain * value


def add_kick(buffer: array[float], start: int, amplitude: float) -> None:
    length = round(BEAT_SAMPLES * 0.32)
    phase = 0.0
    for local in range(min(length, TOTAL_SAMPLES - start)):
        progress = local / max(1, length - 1)
        frequency = 118.0 * (48.0 / 118.0) ** progress
        phase += TAU * frequency / SAMPLE_RATE
        gain = (1.0 - progress) ** 3.2
        buffer[start + local] += amplitude * gain * math.sin(phase)


def add_noise_hit(
    buffer: array[float], start: int, beats: float, amplitude: float, seed: int
) -> None:
    length = round(BEAT_SAMPLES * beats)
    state = seed & 0x7FFF_FFFF
    previous = 0.0
    for local in range(min(length, TOTAL_SAMPLES - start)):
        state = (1_103_515_245 * state + 12_345) & 0x7FFF_FFFF
        noise = state / 0x3FFF_FFFF - 1.0
        high = noise - previous * 0.82
        previous = noise
        progress = local / max(1, length - 1)
        gain = (1.0 - progress) ** 2.5
        buffer[start + local] += amplitude * gain * high


def add_metal_tick(
    buffer: array[float], start: int, amplitude: float, frequency: float
) -> None:
    length = round(BEAT_SAMPLES * 0.10)
    for local in range(min(length, TOTAL_SAMPLES - start)):
        progress = local / max(1, length - 1)
        phase = TAU * frequency * local / SAMPLE_RATE
        value = math.sin(phase) + 0.55 * math.sin(phase * 2.71 + 0.4)
        buffer[start + local] += amplitude * (1.0 - progress) ** 2.8 * value / 1.4


def add_chord(
    buffer: array[float], bar: int, notes: tuple[int, ...], amplitude: float
) -> None:
    start = sample_at(bar, 0.0)
    length = round(BEAT_SAMPLES * 3.45)
    for index, note in enumerate(notes):
        add_tone(
            buffer,
            start,
            length,
            note,
            amplitude / len(notes),
            "air",
            attack=0.18 + index * 0.03,
            release=0.28,
        )


def render_stems() -> dict[str, array[float]]:
    stems = {
        name: array("f", [0.0]) * TOTAL_SAMPLES
        for name in STEMS
    }
    base = stems["music_base"]
    drive = stems["music_drive"]
    pressure = stems["music_pressure"]

    roots = (38, 43, 36, 45, 38, 47, 41, 36)
    bass_intervals = (0, 0, 5, 0, 10)
    bass_beats = (0.0, 0.75, 1.5, 2.5, 3.25)
    motif = (74, 79, 76, 70, 72)
    motif_beats = (0.0, 0.75, 1.5, 2.75, 3.25)

    for bar in range(BARS):
        root = roots[(bar // 2) % len(roots)]
        final_bar = bar == BARS - 1
        for beat, interval in zip(bass_beats, bass_intervals, strict=True):
            if final_bar and beat >= 2.5:
                continue
            add_tone(
                base,
                sample_at(bar, beat),
                round(BEAT_SAMPLES * 0.42),
                root + interval,
                0.21,
                "sub",
                attack=0.004,
                release=0.10,
            )
        add_kick(base, sample_at(bar, 0.0), 0.10)
        if not final_bar:
            add_metal_tick(base, sample_at(bar, 2.0), 0.035, 760.0)

        if bar % 4 == 0 and bar < BARS - 2:
            add_chord(base, bar, (root + 12, root + 17, root + 22), 0.15)

        if bar % 8 in (2, 6) and not final_bar:
            transform = (bar // 8) % 4
            notes = motif if transform % 2 == 0 else tuple(reversed(motif))
            transpose = (0, -2, 3, -5)[transform]
            for beat, note in zip(motif_beats, notes, strict=True):
                add_tone(
                    base,
                    sample_at(bar, beat),
                    round(BEAT_SAMPLES * 0.31),
                    note + transpose,
                    0.10,
                    "glass",
                    attack=0.006,
                    release=0.07,
                )

        for beat in (0.0, 2.0):
            add_kick(drive, sample_at(bar, beat), 0.24)
        for beat in (1.0, 3.0):
            if final_bar and beat >= 3.0:
                continue
            add_noise_hit(
                drive,
                sample_at(bar, beat),
                0.22,
                0.105,
                seed=bar * 97 + round(beat * 31) + 17,
            )
        for step in range(8):
            beat = step * 0.5
            if final_bar and beat >= 3.0:
                continue
            add_noise_hit(
                drive,
                sample_at(bar, beat),
                0.075,
                0.026 if step % 2 else 0.038,
                seed=bar * 131 + step * 19 + 53,
            )
        arpeggio = (root + 24, root + 29, root + 34, root + 27, root + 31)
        for beat, note in zip(bass_beats, arpeggio, strict=True):
            if final_bar and beat >= 2.5:
                continue
            add_tone(
                drive,
                sample_at(bar, beat),
                round(BEAT_SAMPLES * 0.18),
                note,
                0.075,
                "pulse",
                attack=0.002,
                release=0.04,
            )

        for step in range(16):
            beat = step * 0.25
            if final_bar and beat >= 2.75:
                continue
            accent = step % 8 in (0, 3, 6)
            add_noise_hit(
                pressure,
                sample_at(bar, beat),
                0.045,
                0.021 if accent else 0.012,
                seed=bar * 211 + step * 43 + 101,
            )
        for beat, frequency in ((0.0, 1_180.0), (1.5, 1_540.0), (3.0, 1_330.0)):
            if final_bar and beat >= 3.0:
                continue
            add_metal_tick(pressure, sample_at(bar, beat), 0.055, frequency)
        if bar % 2 == 1 and not final_bar:
            counter = (81, 76, 84, 78)
            for beat, note in zip((0.5, 1.75, 2.5, 3.25), counter, strict=True):
                add_tone(
                    pressure,
                    sample_at(bar, beat),
                    round(BEAT_SAMPLES * 0.20),
                    note + ((bar // 8) % 3 - 1) * 2,
                    0.064,
                    "glass",
                    attack=0.003,
                    release=0.05,
                )

    combined_peak = max(
        abs(base[index] + drive[index] + pressure[index])
        for index in range(TOTAL_SAMPLES)
    )
    if combined_peak > 0.92:
        scale = 0.92 / combined_peak
        for buffer in stems.values():
            for index in range(TOTAL_SAMPLES):
                buffer[index] *= scale
    return stems


def pcm_bytes(buffer: array[float]) -> bytes:
    samples = array(
        "h",
        (
            round(max(-1.0, min(1.0, value)) * 32_767)
            for value in buffer
        ),
    )
    if sys.byteorder != "little":
        samples.byteswap()
    return samples.tobytes()


def wav_bytes(buffer: array[float]) -> bytes:
    pcm = pcm_bytes(buffer)
    output = bytearray(44 + len(pcm))
    data_size = len(pcm)
    struct.pack_into("<4sI4s", output, 0, b"RIFF", 36 + data_size, b"WAVE")
    struct.pack_into(
        "<4sIHHIIHH",
        output,
        12,
        b"fmt ",
        16,
        1,
        1,
        SAMPLE_RATE,
        SAMPLE_RATE * 2,
        2,
        16,
    )
    struct.pack_into("<4sI", output, 36, b"data", data_size)
    output[44:] = pcm
    return bytes(output)


def encode_ogg(master: Path, output: Path) -> str:
    output.unlink(missing_ok=True)
    if ffmpeg := shutil.which("ffmpeg"):
        command = [
            ffmpeg,
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            str(master),
            "-map_metadata",
            "-1",
            "-ar",
            str(SAMPLE_RATE),
            "-ac",
            "1",
            "-c:a",
            "libvorbis",
            "-q:a",
            "3",
            str(output),
        ]
        subprocess.run(command, check=True)
        return "ffmpeg-libvorbis-q3"
    cargo = shutil.which("cargo")
    encoder_manifest = ROOT / "tools" / "music_encoder" / "Cargo.toml"
    if cargo and encoder_manifest.is_file():
        command = [
            cargo,
            "run",
            "--quiet",
            "--release",
            "--manifest-path",
            str(encoder_manifest),
            "--target-dir",
            str(ROOT / "target" / "music-encoder"),
            "--",
            str(master),
            str(output),
        ]
        subprocess.run(command, check=True)
        return "vorbis-rs-quality-vbr-0.35"
    raise RuntimeError(
        "FFmpeg or the repository's Rust music encoder is required "
        "to encode music stems."
    )


def stem_metrics(buffer: array[float], master: bytes) -> dict[str, object]:
    peak = max(abs(value) for value in buffer)
    rms = math.sqrt(sum(value * value for value in buffer) / len(buffer))
    return {
        "master_sha256": hashlib.sha256(master).hexdigest(),
        "peak": round(peak, 6),
        "rms": round(rms, 6),
    }


def expected_manifest(stems: dict[str, array[float]]) -> dict[str, object]:
    stem_data: dict[str, object] = {}
    for name, buffer in stems.items():
        master = wav_bytes(buffer)
        stem_data[name] = stem_metrics(buffer, master)
    return {
        "format_version": 1,
        "sample_rate": SAMPLE_RATE,
        "bpm": BPM,
        "beat_samples": BEAT_SAMPLES,
        "bar_samples": BAR_SAMPLES,
        "bars": BARS,
        "total_samples": TOTAL_SAMPLES,
        "duration_seconds": round(TOTAL_SAMPLES / SAMPLE_RATE, 6),
        "budget_bytes": MUSIC_BUDGET_BYTES,
        "stems": stem_data,
    }


def generate(stems: dict[str, array[float]]) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    MASTER_DIR.mkdir(parents=True, exist_ok=True)
    manifest = expected_manifest(stems)
    encoders: set[str] = set()
    total_bytes = 0
    for name, buffer in stems.items():
        master_path = MASTER_DIR / f"{name}.wav"
        asset_path = OUTPUT_DIR / f"{name}.ogg"
        master_path.write_bytes(wav_bytes(buffer))
        encoders.add(encode_ogg(master_path, asset_path))
        asset = asset_path.read_bytes()
        total_bytes += len(asset)
        manifest["stems"][name].update(
            {
                "asset": asset_path.name,
                "asset_bytes": len(asset),
                "asset_sha256": hashlib.sha256(asset).hexdigest(),
            }
        )
    manifest["encoder"] = ",".join(sorted(encoders))
    manifest["total_asset_bytes"] = total_bytes
    if total_bytes > MUSIC_BUDGET_BYTES:
        raise RuntimeError(
            f"Music assets use {total_bytes} bytes; budget is {MUSIC_BUDGET_BYTES}."
        )
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(
        f"Generated {len(STEMS)} music stems "
        f"({total_bytes / (1024 * 1024):.2f} MiB, {TOTAL_SAMPLES / SAMPLE_RATE:.2f}s)."
    )


def check(stems: dict[str, array[float]]) -> int:
    if not MANIFEST_PATH.is_file():
        print("Music manifest is missing.", file=sys.stderr)
        return 1
    recorded = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    expected = expected_manifest(stems)
    mismatches: list[str] = []
    for key, value in expected.items():
        if key == "stems":
            continue
        if recorded.get(key) != value:
            mismatches.append(key)
    for name in STEMS:
        path = OUTPUT_DIR / f"{name}.ogg"
        expected_stem = expected["stems"][name]
        recorded_stem = recorded.get("stems", {}).get(name, {})
        if recorded_stem.get("master_sha256") != expected_stem["master_sha256"]:
            mismatches.append(f"{name}:master")
        if not path.is_file() or not path.read_bytes().startswith(b"OggS"):
            mismatches.append(f"{name}:asset")
            continue
        asset = path.read_bytes()
        if recorded_stem.get("asset_sha256") != hashlib.sha256(asset).hexdigest():
            mismatches.append(f"{name}:asset-sha256")
        if recorded_stem.get("asset_bytes") != len(asset):
            mismatches.append(f"{name}:asset-bytes")
    total_bytes = sum(
        (OUTPUT_DIR / f"{name}.ogg").stat().st_size
        for name in STEMS
        if (OUTPUT_DIR / f"{name}.ogg").is_file()
    )
    if total_bytes > MUSIC_BUDGET_BYTES:
        mismatches.append("asset-budget")
    if recorded.get("total_asset_bytes") != total_bytes:
        mismatches.append("total-asset-bytes")
    if mismatches:
        print("Music assets are stale: " + ", ".join(mismatches), file=sys.stderr)
        return 1
    print(
        f"Verified {len(STEMS)} deterministic music stems "
        f"({total_bytes / (1024 * 1024):.2f} MiB)."
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the checked-in stems and manifest",
    )
    args = parser.parse_args()
    stems = render_stems()
    if args.check:
        return check(stems)
    generate(stems)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
