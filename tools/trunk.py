#!/usr/bin/env python3
"""Run Trunk with the repository's rustup-managed compiler."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def rustup_tool(toolchain: str, tool: str) -> str:
    result = subprocess.run(
        ["rustup", "which", "--toolchain", toolchain, tool],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def main() -> int:
    if shutil.which("rustup") is None:
        print("rustup is required to build Oxidefall", file=sys.stderr)
        return 1
    if shutil.which("trunk") is None:
        print("Trunk is required to build the web app", file=sys.stderr)
        return 1
    toolchain_file = ROOT / "rust-toolchain.toml"
    toolchain = tomllib.loads(toolchain_file.read_text())["toolchain"]["channel"]
    environment = os.environ.copy()
    environment.pop("NO_COLOR", None)
    environment.update(
        {
            "CARGO": rustup_tool(toolchain, "cargo"),
            "RUSTC": rustup_tool(toolchain, "rustc"),
            "RUSTDOC": rustup_tool(toolchain, "rustdoc"),
            "TRUNK_COLOR": "never",
        }
    )
    os.execvpe("trunk", ["trunk", *sys.argv[1:]], environment)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
