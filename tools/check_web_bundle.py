#!/usr/bin/env python3
"""Enforce Oxidefall's compressed critical-path budget."""

from __future__ import annotations

import gzip
import sys
from pathlib import Path


LIMIT_BYTES = 3 * 1024 * 1024


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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
