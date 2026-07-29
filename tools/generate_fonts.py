#!/usr/bin/env python3
"""Generate and verify Oxidefall's renamed, UI-specific font subsets."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FONT_DIR = ROOT / "assets" / "fonts"
SOURCE_DIR = FONT_DIR / "source"
MANIFEST_PATH = FONT_DIR / "font_manifest.json"
UNICODE_SPEC = "U+0020-007E,U+00A0-00FF,U+2010-2027,U+2190-2195,U+21A8"
FONTS = (
    ("SairaCondensed-ExtraBold.ttf", "OxidefallDisplay.ttf"),
    ("IBMPlexMono-Medium.ttf", "OxidefallMono.ttf"),
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def required_codepoints() -> set[int]:
    codepoints = set(range(0x20, 0x7F))
    sources = [*ROOT.joinpath("src").rglob("*.rs"), ROOT / "index.html"]
    for source in sources:
        codepoints.update(
            ord(character)
            for character in source.read_text()
            if ord(character) > 0x7F
        )
    return codepoints


def cmap_subtables(data: bytes) -> list[int]:
    table_count = struct.unpack_from(">H", data, 4)[0]
    cmap_offset = None
    for index in range(table_count):
        record = 12 + index * 16
        if data[record : record + 4] == b"cmap":
            cmap_offset = struct.unpack_from(">I", data, record + 8)[0]
            break
    if cmap_offset is None:
        return []
    subtable_count = struct.unpack_from(">H", data, cmap_offset + 2)[0]
    return [
        cmap_offset + struct.unpack_from(">I", data, cmap_offset + 4 + index * 8 + 4)[0]
        for index in range(subtable_count)
    ]


def format_four_has(data: bytes, offset: int, codepoint: int) -> bool:
    if codepoint > 0xFFFF:
        return False
    segment_count = struct.unpack_from(">H", data, offset + 6)[0] // 2
    end_offset = offset + 14
    start_offset = end_offset + segment_count * 2 + 2
    delta_offset = start_offset + segment_count * 2
    range_offset = delta_offset + segment_count * 2
    for index in range(segment_count):
        end = struct.unpack_from(">H", data, end_offset + index * 2)[0]
        start = struct.unpack_from(">H", data, start_offset + index * 2)[0]
        if not start <= codepoint <= end:
            continue
        delta = struct.unpack_from(">h", data, delta_offset + index * 2)[0]
        range_word = range_offset + index * 2
        distance = struct.unpack_from(">H", data, range_word)[0]
        if distance == 0:
            return (codepoint + delta) & 0xFFFF != 0
        glyph_offset = range_word + distance + (codepoint - start) * 2
        if glyph_offset + 2 > len(data):
            return False
        glyph = struct.unpack_from(">H", data, glyph_offset)[0]
        return glyph != 0 and ((glyph + delta) & 0xFFFF) != 0
    return False


def format_twelve_has(data: bytes, offset: int, codepoint: int) -> bool:
    group_count = struct.unpack_from(">I", data, offset + 12)[0]
    for index in range(group_count):
        group = offset + 16 + index * 12
        start, end, first_glyph = struct.unpack_from(">III", data, group)
        if start <= codepoint <= end:
            return first_glyph + codepoint - start != 0
        if codepoint < start:
            return False
    return False


def font_has(data: bytes, codepoint: int) -> bool:
    for offset in cmap_subtables(data):
        format_number = struct.unpack_from(">H", data, offset)[0]
        if format_number == 4 and format_four_has(data, offset, codepoint):
            return True
        if format_number == 12 and format_twelve_has(data, offset, codepoint):
            return True
    return False


def manifest() -> dict[str, object]:
    fonts = {}
    for source_name, subset_name in FONTS:
        source = SOURCE_DIR / source_name
        subset = FONT_DIR / subset_name
        fonts[subset_name] = {
            "source": source_name,
            "source_sha256": digest(source),
            "subset_sha256": digest(subset),
            "subset_bytes": subset.stat().st_size,
        }
    return {"format_version": 1, "unicode_spec": UNICODE_SPEC, "fonts": fonts}


def generate() -> None:
    subsetter = shutil.which("hb-subset")
    if subsetter is None:
        raise RuntimeError("HarfBuzz hb-subset is required to regenerate fonts")
    with tempfile.TemporaryDirectory() as temporary:
        temporary_dir = Path(temporary)
        for source_name, subset_name in FONTS:
            generated = temporary_dir / subset_name
            subprocess.run(
                [
                    subsetter,
                    str(SOURCE_DIR / source_name),
                    f"--unicodes={UNICODE_SPEC}",
                    "--name-IDs=",
                    f"--output-file={generated}",
                ],
                check=True,
            )
            shutil.copyfile(generated, FONT_DIR / subset_name)
    MANIFEST_PATH.write_text(json.dumps(manifest(), indent=2) + "\n")
    print("Generated 2 renamed font subsets.")


def check() -> int:
    if not MANIFEST_PATH.is_file():
        print("Font manifest is missing.")
        return 1
    recorded = json.loads(MANIFEST_PATH.read_text())
    current = manifest()
    failures = [] if recorded == current else ["manifest"]
    required = required_codepoints()
    for _, subset_name in FONTS:
        data = (FONT_DIR / subset_name).read_bytes()
        missing = sorted(codepoint for codepoint in required if not font_has(data, codepoint))
        if missing:
            failures.append(
                f"{subset_name}:missing-" + ",".join(f"U+{value:04X}" for value in missing)
            )
    if failures:
        print("Font assets are stale: " + "; ".join(failures))
        return 1
    print("Verified 2 font subsets and UI glyph coverage.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        return check()
    generate()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
