#!/usr/bin/env python3
"""Resolve Cargo package license metadata for release preflight."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def load_metadata(args: argparse.Namespace) -> str:
    if args.metadata_json:
        return Path(args.metadata_json).read_text(encoding="utf-8")
    return subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        text=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--metadata-json", type=Path)
    args = parser.parse_args()

    metadata = json.loads(load_metadata(args))
    package = metadata["packages"][0]
    package_license = package.get("license") or ""
    package_license_file = package.get("license_file") or ""
    if not package_license and not package_license_file:
        raise SystemExit("Cargo package metadata must declare license or license_file")

    print(package_license)
    print(package_license_file)
    print("true")


if __name__ == "__main__":
    main()
