#!/usr/bin/env python3
"""Smoke tests for Cargo license metadata release preflight helper."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github" / "scripts" / "resolve-cargo-license-metadata.py"


def resolve(package: dict[str, str]) -> list[str]:
    metadata = {"packages": [package]}
    with tempfile.TemporaryDirectory() as tmp:
        metadata_path = Path(tmp) / "metadata.json"
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        result = subprocess.run(
            ["python3", str(SCRIPT), "--metadata-json", str(metadata_path)],
            check=True,
            text=True,
            capture_output=True,
        )
    return result.stdout.splitlines()


def main() -> None:
    assert resolve({"license": "MIT"}) == ["MIT", "", "true"]
    assert resolve({"license_file": "LICENSE"}) == ["", "LICENSE", "true"]
    assert resolve({}) == ["", "", "false"]


if __name__ == "__main__":
    main()
