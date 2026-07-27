#!/usr/bin/env python3
"""Generate Ferrofall's deterministic, original sound-effect bank."""

from __future__ import annotations

import argparse
import io
import math
import random
import struct
import wave
from collections.abc import Callable
from pathlib import Path


SAMPLE_RATE = 32_000
TAU = math.tau
ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "assets" / "audio"
SampleFunction = Callable[[float, float], float]


def envelope(t: float, start: float, duration: float, attack: float = 0.006) -> float:
    local = t - start
    if local < 0.0 or local >= duration:
        return 0.0
    attack_gain = min(1.0, local / max(attack, 0.000_1))
    return attack_gain * (1.0 - local / duration) ** 1.7


def tone(
    t: float,
    start: float,
    duration: float,
    frequency: float,
    end_frequency: float | None = None,
    amplitude: float = 1.0,
    metallic: float = 0.0,
) -> float:
    gain = envelope(t, start, duration)
    if gain == 0.0:
        return 0.0
    local = t - start
    end = frequency if end_frequency is None else end_frequency
    slope = (end - frequency) / duration
    phase = TAU * (frequency * local + 0.5 * slope * local * local)
    fundamental = math.sin(phase)
    upper = math.sin(phase * 2.71 + 0.4) * metallic
    return amplitude * gain * (fundamental + upper) / (1.0 + metallic)


def pulse(
    t: float,
    start: float,
    duration: float,
    frequency: float,
    amplitude: float = 1.0,
) -> float:
    gain = envelope(t, start, duration, attack=0.0015)
    if gain == 0.0:
        return 0.0
    phase = TAU * frequency * (t - start)
    softened_square = math.sin(phase) + math.sin(phase * 3.0) / 5.0
    return amplitude * gain * softened_square / 1.2


def noise_burst(
    t: float,
    noise: float,
    start: float,
    duration: float,
    amplitude: float,
) -> float:
    return noise * amplitude * envelope(t, start, duration, attack=0.000_5)


def sequence(
    notes: list[tuple[float, float]],
    note_duration: float,
    gap: float,
    amplitude: float = 0.65,
    metallic: float = 0.18,
) -> tuple[float, SampleFunction]:
    total = notes[-1][0] + note_duration + 0.05

    def sample(t: float, _noise: float) -> float:
        return sum(
            tone(
                t,
                offset,
                note_duration,
                frequency,
                amplitude=amplitude,
                metallic=metallic,
            )
            for offset, frequency in notes
        )

    return max(total, gap), sample


def sound_recipes() -> dict[str, tuple[float, SampleFunction]]:
    recipes: dict[str, tuple[float, SampleFunction]] = {}

    recipes["ui_activate"] = (
        0.07,
        lambda t, n: pulse(t, 0.0, 0.055, 1_080.0, 0.42)
        + noise_burst(t, n, 0.0, 0.018, 0.10),
    )
    recipes["game_start"] = sequence(
        [(0.00, 220.0), (0.07, 330.0), (0.14, 495.0), (0.22, 660.0)],
        0.15,
        0.42,
        amplitude=0.48,
        metallic=0.28,
    )
    recipes["pause"] = (
        0.18,
        lambda t, n: tone(t, 0.0, 0.16, 560.0, 260.0, 0.55, 0.22)
        + noise_burst(t, n, 0.0, 0.025, 0.08),
    )
    recipes["resume"] = (
        0.18,
        lambda t, n: tone(t, 0.0, 0.16, 280.0, 620.0, 0.50, 0.20)
        + tone(t, 0.075, 0.09, 760.0, amplitude=0.24),
    )
    recipes["move_a"] = (
        0.065,
        lambda t, n: pulse(t, 0.0, 0.048, 720.0, 0.36)
        + noise_burst(t, n, 0.0, 0.020, 0.11),
    )
    recipes["move_b"] = (
        0.065,
        lambda t, n: pulse(t, 0.0, 0.048, 790.0, 0.34)
        + noise_burst(t, n, 0.0, 0.017, 0.10),
    )
    recipes["rotate"] = (
        0.13,
        lambda t, n: tone(t, 0.0, 0.105, 430.0, 860.0, 0.52, 0.34)
        + pulse(t, 0.055, 0.055, 1_120.0, 0.20)
        + noise_burst(t, n, 0.0, 0.022, 0.09),
    )
    recipes["hold"] = (
        0.22,
        lambda t, n: tone(t, 0.0, 0.17, 510.0, 255.0, 0.46, 0.38)
        + tone(t, 0.07, 0.13, 760.0, 960.0, 0.34, 0.20),
    )
    recipes["contact"] = (
        0.075,
        lambda t, n: tone(t, 0.0, 0.060, 175.0, 120.0, 0.34, 0.45)
        + noise_burst(t, n, 0.0, 0.030, 0.18),
    )
    recipes["hard_drop"] = (
        0.17,
        lambda t, n: tone(t, 0.0, 0.14, 260.0, 70.0, 0.68, 0.46)
        + noise_burst(t, n, 0.0, 0.065, 0.34),
    )
    recipes["lock"] = (
        0.14,
        lambda t, n: tone(t, 0.0, 0.11, 205.0, 105.0, 0.55, 0.52)
        + pulse(t, 0.0, 0.055, 92.0, 0.28)
        + noise_burst(t, n, 0.0, 0.045, 0.24),
    )

    recipes["clear_single"] = sequence(
        [(0.00, 440.0), (0.075, 660.0)], 0.20, 0.32, amplitude=0.48
    )
    recipes["clear_double"] = sequence(
        [(0.00, 440.0), (0.07, 660.0), (0.14, 880.0)],
        0.22,
        0.40,
        amplitude=0.49,
    )
    recipes["clear_triple"] = sequence(
        [(0.00, 392.0), (0.065, 587.3), (0.13, 784.0), (0.205, 1_046.5)],
        0.25,
        0.52,
        amplitude=0.48,
        metallic=0.24,
    )
    recipes["clear_four"] = sequence(
        [
            (0.00, 330.0),
            (0.065, 495.0),
            (0.13, 660.0),
            (0.20, 990.0),
            (0.29, 1_320.0),
        ],
        0.32,
        0.70,
        amplitude=0.47,
        metallic=0.34,
    )
    recipes["accent_tspin"] = (
        0.34,
        lambda t, n: tone(t, 0.0, 0.28, 310.0, 1_240.0, 0.46, 0.58)
        + pulse(t, 0.12, 0.14, 930.0, 0.22),
    )
    recipes["accent_combo"] = (
        0.17,
        lambda t, n: pulse(t, 0.0, 0.12, 1_040.0, 0.34)
        + tone(t, 0.045, 0.10, 1_560.0, amplitude=0.22),
    )
    recipes["accent_back_to_back"] = sequence(
        [(0.00, 740.0), (0.055, 1_110.0), (0.11, 1_480.0)],
        0.14,
        0.28,
        amplitude=0.34,
        metallic=0.32,
    )
    recipes["accent_perfect"] = sequence(
        [
            (0.00, 523.3),
            (0.08, 659.3),
            (0.16, 784.0),
            (0.24, 1_046.5),
            (0.34, 1_318.5),
        ],
        0.30,
        0.72,
        amplitude=0.42,
        metallic=0.16,
    )
    recipes["level_up"] = sequence(
        [(0.00, 440.0), (0.09, 554.4), (0.18, 659.3), (0.28, 880.0)],
        0.24,
        0.58,
        amplitude=0.42,
        metallic=0.26,
    )
    recipes["game_over"] = (
        1.10,
        lambda t, n: tone(t, 0.00, 0.42, 440.0, 220.0, 0.52, 0.32)
        + tone(t, 0.25, 0.42, 330.0, 165.0, 0.48, 0.35)
        + tone(t, 0.50, 0.52, 220.0, 82.5, 0.52, 0.46)
        + noise_burst(t, n, 0.50, 0.34, 0.08),
    )
    recipes["new_best"] = sequence(
        [
            (0.00, 392.0),
            (0.10, 523.3),
            (0.20, 659.3),
            (0.31, 784.0),
            (0.46, 1_046.5),
            (0.66, 1_318.5),
        ],
        0.42,
        1.24,
        amplitude=0.44,
        metallic=0.22,
    )
    return recipes


def render_wav(name: str, duration: float, sample: SampleFunction) -> bytes:
    random_source = random.Random(f"ferrofall-audio-v1:{name}")
    samples = [
        sample(index / SAMPLE_RATE, random_source.uniform(-1.0, 1.0))
        for index in range(round(duration * SAMPLE_RATE))
    ]
    peak = max((abs(value) for value in samples), default=1.0)
    scale = min(0.88 / max(peak, 0.000_1), 1.0)
    pcm = b"".join(
        struct.pack("<h", round(max(-1.0, min(1.0, value * scale)) * 32_767))
        for value in samples
    )

    output = io.BytesIO()
    with wave.open(output, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        wav.writeframes(pcm)
    return output.getvalue()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify that checked-in WAV files match the generator",
    )
    args = parser.parse_args()
    recipes = sound_recipes()
    mismatches: list[str] = []

    if not args.check:
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    for name, (duration, sample) in sorted(recipes.items()):
        path = OUTPUT_DIR / f"{name}.wav"
        generated = render_wav(name, duration, sample)
        if args.check:
            if not path.exists() or path.read_bytes() != generated:
                mismatches.append(path.name)
        else:
            path.write_bytes(generated)

    if mismatches:
        print("Audio assets are stale: " + ", ".join(mismatches))
        return 1
    if args.check:
        print(f"Verified {len(recipes)} deterministic audio assets.")
    else:
        total = sum((OUTPUT_DIR / f"{name}.wav").stat().st_size for name in recipes)
        print(f"Generated {len(recipes)} audio assets ({total / 1024:.1f} KiB).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
